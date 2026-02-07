use std::{
  io,
  pin::Pin,
  task::{Context, Poll},
};

use futures_core::Stream;
use hyper::client::connect::{Connected as HyperClientConnected, Connection as HyperConnection};

use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf, duplex};
use tokio::sync::mpsc;
use tonic::transport::server::Connected;

pub struct InMemoryIO {
  rx: mpsc::Receiver<InMemoryConnection>,
}

impl Default for InMemoryIO {
  fn default() -> Self {
    InMemoryIO::new_pair().0
  }
}

pub struct InMemoryDialer {
  tx: mpsc::Sender<InMemoryConnection>,
}

impl InMemoryIO {
  pub fn new_pair() -> (Self, InMemoryDialer) {
    InMemoryIO::new_pair_with_capacity(128)
  }

  pub fn new_pair_with_capacity(capacity: usize) -> (Self, InMemoryDialer) {
    let (tx, rx) = mpsc::channel(capacity);
    (InMemoryIO { rx }, InMemoryDialer { tx })
  }
}

impl Stream for InMemoryIO {
  type Item = Result<InMemoryConnection, io::Error>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    match Pin::new(&mut self.rx).poll_recv(cx) {
      Poll::Ready(Some(conn)) => Poll::Ready(Some(Ok(conn))),
      Poll::Ready(None) => Poll::Ready(None),
      Poll::Pending => Poll::Pending,
    }
  }
}

pub struct InMemoryConnection {
  pub server_half: DuplexStream,
}

impl Unpin for InMemoryConnection {}

impl AsyncRead for InMemoryConnection {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<Result<(), io::Error>> {
    let connection = self.get_mut();
    Pin::new(&mut connection.server_half).poll_read(cx, buf)
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

  fn connect_info(&self) -> Self::ConnectInfo {}
}

pub struct ClientConn {
  inner: DuplexStream,
}

impl Unpin for ClientConn {}

impl AsyncRead for ClientConn {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<Result<(), io::Error>> {
    let c = self.get_mut();
    Pin::new(&mut c.inner).poll_read(cx, buf)
  }
}

impl AsyncWrite for ClientConn {
  fn poll_write(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<Result<usize, io::Error>> {
    let c = self.get_mut();
    Pin::new(&mut c.inner).poll_write(cx, buf)
  }

  fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
    let c = self.get_mut();
    Pin::new(&mut c.inner).poll_flush(cx)
  }

  fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
    let c = self.get_mut();
    Pin::new(&mut c.inner).poll_shutdown(cx)
  }
}

impl HyperConnection for ClientConn {
  fn connected(&self) -> HyperClientConnected {
    HyperClientConnected::new()
  }
}

impl InMemoryDialer {
  pub fn dial(&self) -> Result<ClientConn, io::Error> {
    let (client_half, server_half) = duplex(64 * 1024);

    let server_conn = InMemoryConnection { server_half };
    match self.tx.try_send(server_conn) {
      Ok(()) => {}
      Err(e) => match e {
        tokio::sync::mpsc::error::TrySendError::Full(_) => {
          return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "in-memory connection queue is full",
          ));
        }
        tokio::sync::mpsc::error::TrySendError::Closed(_) => {
          return Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "failed to send server conn",
          ));
        }
      },
    }

    Ok(ClientConn { inner: client_half })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dial_queue_full() {
    let (_io, dialer) = InMemoryIO::new_pair_with_capacity(1);
    assert!(dialer.dial().is_ok());
    let res = dialer.dial();
    assert!(res.is_err());
    assert_eq!(res.err().unwrap().kind(), io::ErrorKind::WouldBlock);
  }

  #[test]
  fn dial_after_receiver_closed() {
    let (io, dialer) = InMemoryIO::new_pair_with_capacity(1);
    drop(io);
    let res = dialer.dial();
    assert!(res.is_err());
    assert_eq!(res.err().unwrap().kind(), io::ErrorKind::BrokenPipe);
  }
}
