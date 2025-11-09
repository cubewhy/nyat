use crate::helpers::spawn_app;

#[tokio::test]
async fn success_with_provided_username_and_strong_password() {
    let app = spawn_app().await;

    let username = "user0";
    let res = app.register(username, "strong_password").await;

    assert_eq!(res.status().as_u16(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    // the response should contains a token
    assert!(body.as_object().unwrap().get("token").is_some());

    // check the user is really added into the database
    let user = sqlx::query!("SELECT id FROM users WHERE username = $1", username)
        .fetch_optional(&app.db)
        .await
        .unwrap();
    assert!(user.is_some());
}

#[tokio::test]
async fn failure_with_weak_password() {
    let app = spawn_app().await;

    let res = app.register("user0", "weak").await;

    assert_eq!(res.status().as_u16(), 400);
}

#[tokio::test]
async fn failure_with_non_ascii_char_in_username() {
    let app = spawn_app().await;

    let res = app.register("不是ascii", "strong_password").await;

    assert_eq!(res.status().as_u16(), 400);
}

#[tokio::test]
async fn failure_with_non_ascii_char_in_password() {
    let app = spawn_app().await;

    let res = app.register("ascii", "不是ascii-1111111").await;

    assert_eq!(res.status().as_u16(), 400);
}

#[tokio::test]
async fn failure_when_username_duplicated() {
    let app = spawn_app().await;

    app.register("user0", "passwd").await;
    let res = app.register("user0", "passwd").await;

    assert_eq!(res.status().as_u16(), 400);
}
