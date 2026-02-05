use std::{
  io,
  pin::Pin,
  task::{Context, Poll},
};

use futures_core::Stream;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf, duplex};
use tonic::transport::server::Connected;

pub struct InMemoryIO {
  conn: Option<InMemoryConnection>,
}

pub struct InMemoryConnection {
  server_half: DuplexStream,
  client_half: DuplexStream,
}

impl InMemoryIO {
  pub fn new() -> Self {
    let (server_half, client_half) = duplex(1024);

    Self {
      conn: Some(InMemoryConnection {
        server_half,
        client_half,
      }),
    }
  }
}

impl Stream for InMemoryIO {
  type Item = Result<InMemoryConnection, io::Error>;

  fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    if let Some(h) = self.conn.take() {
      Poll::Ready(Some(Ok(InMemoryConnection {
        server_half: h.server_half,
        client_half: h.client_half,
      })))
    } else {
      Poll::Ready(None)
    }
  }
}

impl Unpin for InMemoryConnection {}

impl AsyncRead for InMemoryConnection {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<Result<(), io::Error>> {
    let connection = self.get_mut();
    Pin::new(&mut connection.client_half).poll_read(cx, buf)
  }
}

impl AsyncWrite for InMemoryConnection {
  fn poll_write(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<Result<usize, io::Error>> {
    let connection = self.get_mut();
    Pin::new(&mut connection.server_half).poll_write(cx, buf)
  }

  fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
    let connection = self.get_mut();
    Pin::new(&mut connection.server_half).poll_flush(cx)
  }

  fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
    let connection = self.get_mut();
    Pin::new(&mut connection.server_half).poll_shutdown(cx)
  }
}

impl Connected for InMemoryConnection {
  type ConnectInfo = ();

  fn connect_info(&self) -> Self::ConnectInfo {
    ()
  }
}
