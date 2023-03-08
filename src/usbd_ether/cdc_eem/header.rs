#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PacketType {
    Data(DataHeader),
    Command(CommandHeader),
    ZeroLength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PacketHeader(u16);

impl PacketHeader {
    fn into_packet_type(self) -> PacketType {
        let bits = self.0;
        if bits == 0 {
            PacketType::ZeroLength
        } else if bits & 0x8000 == 0 {
            PacketType::Data(DataHeader(bits))
        } else {
            PacketType::Command(CommandHeader(bits))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DataHeader(u16);

impl DataHeader {
    fn is_crc_calculated(&self) -> bool {
        self.0 & 0x4000 != 0
    }

    fn length(&self) -> u16 {
        self.0 & 0x3fff
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EemCmd {
    Echo(EchoCmdHeader),
    EchoResponse(EchoResponseCmdHeader),
    SuspendHint(SuspendHintCmdHeader),
    ResponseHint(ResponseHintCmdHeader),
    ResponseCompleteHint(ResponseCompleteHintCmdHeader),
    Tickle(TickleCmdHeader),
    Reserved6,
    Reserved7,
    ReservedTagged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommandHeader(u16);

impl CommandHeader {
    fn eem_cmd(&self) -> EemCmd {
        let bits = self.0;
        if bits & 0x7000 != 0 {
            return EemCmd::ReservedTagged;
        }
        match (bits >> 11) & 0x07 {
            0 => EemCmd::Echo(EchoCmdHeader(bits)),
            1 => EemCmd::EchoResponse(EchoResponseCmdHeader(bits)),
            2 => EemCmd::SuspendHint(SuspendHintCmdHeader(bits)),
            3 => EemCmd::ResponseHint(ResponseHintCmdHeader(bits)),
            4 => EemCmd::ResponseCompleteHint(ResponseCompleteHintCmdHeader(
                bits,
            )),
            5 => EemCmd::Tickle(TickleCmdHeader(bits)),
            6 => EemCmd::Reserved6,
            7 => EemCmd::Reserved7,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EchoCmdHeader(u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EchoResponseCmdHeader(u16);

/// Ignoring this since the host might also ignore it
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SuspendHintCmdHeader(u16);
/// Ignoring this since the host might also ignore it
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResponseHintCmdHeader(u16);
/// Ignoring this since the host might also ignore it
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResponseCompleteHintCmdHeader(u16);
/// Ignoring this since the host might also ignore it
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TickleCmdHeader(u16);
