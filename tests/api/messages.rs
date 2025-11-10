use crate::helpers::spawn_app;

#[tokio::test]
async fn success_with_valid_chat_id() {
    let app = spawn_app().await;

    let peer = app.create_test_user().await;
    // create pm
    let chat_id = app
        .create_pm_returns_id(&app.test_user.token, &peer.username)
        .await;

    // send message
    let res = app
        .send_chat_message(&app.test_user.token, chat_id, "hello world")
        .await;
    assert_eq!(res.status().as_u16(), 201);

    // send message with peer user
    let res = app
        .send_chat_message(&peer.token, chat_id, "hello world")
        .await;
    assert_eq!(res.status().as_u16(), 201);
}

#[tokio::test]
async fn failure_when_no_permission() {
    let app = spawn_app().await;

    let user1 = app.create_test_user().await;
    let user2 = app.create_test_user().await;
    // create pm
    let chat_id = app
        .create_pm_returns_id(&user1.token, &user2.username)
        .await;

    // send message with the test_user (not user1 or user2)
    let res = app
        .send_chat_message(&app.test_user.token, chat_id, "hello world")
        .await;
    assert_eq!(res.status().as_u16(), 403);
}
