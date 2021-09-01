use strum::EnumString;

#[derive(Debug, Clone, Copy, EnumString, PartialEq, PartialOrd, Eq, Ord)]
#[strum(ascii_case_insensitive)]
pub enum Protocol {
    Tcp,
    Udp,
}
