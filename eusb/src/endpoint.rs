use crate::platform::EndpointInImpl;

pub struct EndpointPipIn {
    inner: EndpointInImpl
}

impl From<EndpointInImpl> for EndpointPipIn {
    fn from(value: EndpointInImpl) -> Self {
        Self{
            inner: value
        }
    }
}

