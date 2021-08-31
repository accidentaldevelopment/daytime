use strum::EnumString;

#[derive(Debug, Clone, Copy, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Protocol {
    Tcp,
    Udp,
}
