use crate::platform::{EndpointPipInInner, EndpointPipInImpl};

pub struct EndpointPipIn {
    inner: EndpointPipInImpl
}

impl From<EndpointPipInImpl> for EndpointPipIn {
    fn from(value: EndpointPipInImpl) -> Self {
        Self{
            inner: value
        }
    }
}

impl EndpointPipIn {
    pub async fn next(&mut self) ->Option<Vec<u8>>{
       self.inner.next().await
    }
}

