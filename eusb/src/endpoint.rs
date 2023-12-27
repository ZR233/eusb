use crate::platform::EndpointInImpl;

pub struct EndpointIn{
    inner: EndpointInImpl
}

impl From<EndpointInImpl> for EndpointIn{
    fn from(value: EndpointInImpl) -> Self {
        Self{
            inner: value
        }
    }
}

