use std::io;

pub fn is_io_error_fine(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::UnexpectedEof
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        io::AsyncReadExt,
        net::{TcpListener, TcpStream},
    };

    use crate::util::io::is_io_error_fine;

    #[tokio::test]
    async fn check_stream_close_is_fine() {
        let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();

        let task1 = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            drop(stream);
        });
        let task2 = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let err = s.read_u8().await.unwrap_err();

            assert!(is_io_error_fine(&err));
        });

        let _ = tokio::join!(task1, task2);
    }
}
