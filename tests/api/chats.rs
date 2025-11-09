use crate::helpers::spawn_app;

#[tokio::test]
async fn success_create_pm_with_valid_peer_username() {
    let app = spawn_app().await;

    // create peer user
    let peer_user = app.create_test_user().await;

    // request the create chat api
    let res = app
        .create_pm(&app.test_user.token, &peer_user.username)
        .await;

    assert_eq!(res.status().as_u16(), 201);

    // check there is a chat id field in the response
    let json = res.json::<serde_json::Value>().await.unwrap();

    let chat_id = json.as_object().unwrap().get("chat_id");
    assert!(chat_id.is_some());
    let chat_id = chat_id.unwrap().as_i64().unwrap();

    // make sure there is a chat in the database
    let chat_entity = sqlx::query!("SELECT id FROM chats WHERE id = $1", chat_id)
        .fetch_optional(&app.db)
        .await
        .unwrap();

    assert!(chat_entity.is_some());
}

#[tokio::test]
async fn failure_create_pm_with_invalid_peer_username() {
    let app = spawn_app().await;

    // request the create chat api
    let res = app.create_pm(&app.test_user.token, "user_not_found").await;

    assert_eq!(res.status().as_u16(), 400);
}

#[tokio::test]
async fn return_previous_chat_if_pm_already_exists() {
    let app = spawn_app().await;

    // create peer user
    let peer_user = app.create_test_user().await;

    // request the create chat api
    let res = app
        .create_pm(&app.test_user.token, &peer_user.username)
        .await;

    // make sure the request is success
    assert_eq!(res.status().as_u16(), 201);

    // check there is a chat id field in the response
    let json = res.json::<serde_json::Value>().await.unwrap();

    let pm_id = json.as_object().unwrap().get("chat_id").unwrap();

    // invoke the api again
    let res = app
        .create_pm(&app.test_user.token, &peer_user.username)
        .await;

    // make sure the request is success
    assert_eq!(res.status().as_u16(), 201);

    // check there is a chat id field in the response
    let json = res.json::<serde_json::Value>().await.unwrap();

    let sec_pm_id = json.as_object().unwrap().get("chat_id").unwrap();

    assert_eq!(pm_id, sec_pm_id);
}
