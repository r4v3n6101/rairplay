use std::io;

pub fn remap_io_error_if_needed(res: io::Result<()>) -> io::Result<()> {
    match res {
        Ok(()) => Ok(()),
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::ConnectionReset
            ) =>
        {
            Ok(())
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        io::AsyncReadExt,
        net::{TcpListener, TcpStream},
    };

    use crate::util::io::remap_io_error_if_needed;

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
            let res = s.read_u8().await.map(|_| ());

            assert!(remap_io_error_if_needed(res).is_ok());
        });

        let _ = tokio::join!(task1, task2);
    }
}
