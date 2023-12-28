#[cfg(test)]
pub(crate) mod test{
    use env_logger::*;
    use log::LevelFilter;


    pub(crate) fn init(){
        Builder::new().filter_level(LevelFilter::Debug).init();
    }
}
pub(crate) fn bcd_to_version(bcd: u16) -> Vec<u16> {
    let  bcd_major = (bcd & 0xF000) >> 12;
    let  bcd_minor = (bcd & 0x0F00) >> 8;
    let  bcd_micro = (bcd & 0x00F0) >> 4;
    let  bcd_nano  = (bcd & 0x000F) >> 0;
    vec![bcd_major, bcd_minor, bcd_micro, bcd_nano]
}
