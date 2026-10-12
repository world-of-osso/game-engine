use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::ClientOptions,
};
use windows_ipc_test::Server;

#[tokio::test]
async fn named_pipe_handles_repeated_connections_and_fragmented_frames() {
    let path = format!(r"\\.\pipe\windows-release-test-{}", std::process::id());
    let server = Server::bind(&path).unwrap();
    let task = tokio::spawn(async move {
        for index in 0..2 {
            let (mut connection, ()) = server.accept().await.unwrap();
            let message: (u32, String) = connection.read().await.unwrap();
            assert_eq!(message, (index, "Windows Æ screenshot".into()));
            connection.write(&(index, "ok".to_string())).await.unwrap();
        }
    });
    for index in 0..2 {
        let mut client = ClientOptions::new().open(&path).unwrap();
        let data = rmp_serde::to_vec(&(index, "Windows Æ screenshot")).unwrap();
        for byte in (data.len() as u32).to_le_bytes().into_iter().chain(data) {
            client.write_all(&[byte]).await.unwrap();
        }
        let length = client.read_u32_le().await.unwrap() as usize;
        let mut response = vec![0; length];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(
            rmp_serde::from_slice::<(u32, String)>(&response).unwrap(),
            (index, "ok".into())
        );
    }
    task.await.unwrap();
    assert!(ClientOptions::new().open(&path).is_err());
}

#[tokio::test]
async fn named_pipe_rejects_oversized_frame_before_allocating_payload() {
    let path = format!(r"\\.\pipe\windows-release-size-test-{}", std::process::id());
    let server = Server::bind(&path).unwrap();
    let task = tokio::spawn(async move {
        let (mut connection, ()) = server.accept().await.unwrap();
        let error = connection.read::<String>().await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    });
    let mut client = ClientOptions::new().open(&path).unwrap();
    client
        .write_all(&(4 * 1024 * 1024u32 + 1).to_le_bytes())
        .await
        .unwrap();
    task.await.unwrap();
}
