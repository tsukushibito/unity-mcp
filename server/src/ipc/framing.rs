use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type FramedIo<T> = Framed<T, LengthDelimitedCodec>;

pub fn codec() -> LengthDelimitedCodec {
    // Default settings: big enough for typical Protobuf payloads.
    // You can tweak max frame length if needed.
    LengthDelimitedCodec::new()
}

pub fn into_framed<T>(io: T) -> FramedIo<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    Framed::new(io, codec())
}