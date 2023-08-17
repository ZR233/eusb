pub enum UsbControlRecipient {
    Device,
    SpecifiedInterface,
    Endpoint,
    Other,
    DefaultInterface
}

pub enum UsbControlTransferType {
    Standard,
    Class,
    Vendor,
    Reserved
}

pub enum Endpoint{
    In{num: u8},
    Out{num: u8},
}



