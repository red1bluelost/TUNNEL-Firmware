#![no_main]
#![no_std]

#[rtic::app(
    device = hal::pac,
    peripherals = true,
    dispatchers = [SPI1, SPI2, SPI3]
)]
mod app {
    use hal::{
        pac::{self, TIM3},
        prelude::*,
        rcc, timer,
    };
    use heapless::pool::singleton::Pool;
    use stm32f4xx_hal as hal;

    use tunnel_firmware::{
        dbg, mem, st7580,
        util::{self, Exchange},
    };

    #[shared]
    struct Shared {
        clocks: rcc::Clocks,
        tim3: Option<TIM3>,
    }

    #[local]
    struct Local {
        st7580_interrupt_handler: st7580::InterruptHandler,
        st7580_driver: st7580::Driver,
        st7580_dsender: st7580::DSender,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = timer::MonoTimerUs<pac::TIM2>;

    #[init(
        local = [
            stbuf: [u8; 1 << 12] = util::zeros(),
        ]
    )]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        dbg::init!();
        dbg::println!("init");

        let init::LocalResources { stbuf } = ctx.local;

        let dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(8.MHz()).sysclk(96.MHz()).freeze();

        let mono = dp.TIM2.monotonic_us(&clocks);

        let gpioa = dp.GPIOA.split();
        let gpioc = dp.GPIOC.split();

        mem::POOL::grow(stbuf);

        let (st7580_driver, st7580_dsender, st7580_interrupt_handler) =
            st7580::Builder {
                t_req: gpioa.pa5.into_push_pull_output(),
                resetn: gpioa.pa8.into_push_pull_output(),
                tx_on: gpioc.pc0,
                rx_on: gpioc.pc1,
                usart: dp.USART1,
                usart_tx: gpioa.pa9.into_alternate(),
                usart_rx: gpioa.pa10.into_alternate(),
                now: monotonics::now,
            }
            .split(&clocks);

        plm::spawn().unwrap();

        dbg::println!("init end");
        (
            Shared {
                clocks,
                tim3: Some(dp.TIM3),
            },
            Local {
                st7580_interrupt_handler,
                st7580_driver,
                st7580_dsender,
            },
            init::Monotonics(mono),
        )
    }

    const DATA_OPT: u8 = 0x44;
    const ACK_BUF_SIZE: usize = 17;
    const TRIG_BUF_SIZE: usize = 21;

    #[task(
        priority = 1,
        shared = [ clocks, tim3 ],
        local = [
            st7580_driver,
            st7580_dsender,
            should_init: bool = true,
            trs_buffer: [u8; ACK_BUF_SIZE] = *b"ACK MESSAGE ID: @",
            rcv_buffer: [u8; TRIG_BUF_SIZE] = util::zeros(),
            last_id_rcv: u8 = 0,
            iter_cntr: i32 = 0,
        ]
    )]
    fn plm(ctx: plm::Context) {
        let plm::LocalResources {
            st7580_driver: driver,
            st7580_dsender: dsender,
            should_init,
            trs_buffer,
            rcv_buffer,
            last_id_rcv,
            iter_cntr,
        } = ctx.local;
        let plm::SharedResources { clocks, tim3 } = ctx.shared;

        let mut delay =
            (clocks, tim3).lock(|c, t| t.exchange(None).unwrap().delay_us(c));

        // We must perform the initialization stage here due to the `init`
        // task being interrupt free.
        if *should_init {
            dbg::println!("plm init");
            driver.init(&mut delay);

            dbg::println!("plm modem conf");
            driver
                .mib_write(st7580::MIB_MODEM_CONF, &st7580::MODEM_CONFIG)
                .and_then(|tag| dsender.enqueue(tag))
                .and_then(|d| nb::block!(d.process()))
                .unwrap();
            delay.delay(500.millis());

            dbg::println!("plm phy conf");
            driver
                .mib_write(st7580::MIB_PHY_CONF, &st7580::PHY_CONFIG)
                .and_then(|tag| dsender.enqueue(tag))
                .and_then(|d| nb::block!(d.process()))
                .unwrap();
            delay.delay(500.millis());
            driver.set_ready_to_receive();

            dbg::println!("P2P Communication Test - Follower Board Side");
            dbg::println!();

            *should_init = false;
        }

        dbg::println!("Iteration {}", *iter_cntr);
        *iter_cntr += 1;

        // Receive Trigger Msg from M board
        let rx_frame = loop {
            match driver.receive_frame() {
                Some(f)
                    if f.stx != st7580::STX_03
                        || *last_id_rcv != f.data[3 + TRIG_BUF_SIZE] =>
                {
                    *last_id_rcv = f.data[3 + TRIG_BUF_SIZE];
                    break f;
                }
                Some(_) | None => {
                    delay.delay(200.millis());
                }
            }
        };

        rcv_buffer.copy_from_slice(&rx_frame.data[4..rx_frame.length as usize]);

        let rcv_last = *rcv_buffer.last().unwrap();
        dbg::println!("Trigger Msg Received, ID: {}", rcv_last as char);
        dbg::println!("PAYLOAD: {}", unsafe {
            core::str::from_utf8_unchecked(rcv_buffer)
        });

        // Send back ACK Msg to Master Board
        *trs_buffer.last_mut().unwrap() = rcv_last;
        loop {
            let buf = mem::alloc_from_slice(trs_buffer).unwrap();
            if driver
                .dl_data(DATA_OPT, buf)
                .and_then(|tag| dsender.enqueue(tag))
                .and_then(|d| nb::block!(d.process()))
                .is_ok()
            {
                break;
            }
        }

        dbg::println!(
            "ACK Msg Sent, ID: {}",
            *trs_buffer.last().unwrap() as char
        );
        dbg::println!("PAYLOAD: {}", unsafe {
            core::str::from_utf8_unchecked(trs_buffer)
        });
        dbg::println!();

        plm::spawn_after(1.secs()).unwrap();
    }

    #[task(binds = USART1, priority = 2, local = [st7580_interrupt_handler])]
    fn usart1(ctx: usart1::Context) {
        ctx.local.st7580_interrupt_handler.handle();
    }
}
