#[cfg(test)]
pub(crate) mod test{
    use env_logger::*;
    use log::LevelFilter;


    pub(crate) fn init(){
        Builder::new().filter_level(LevelFilter::Debug).init();
    }
}