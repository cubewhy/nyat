use crate::helpers::spawn_app;

#[tokio::test]
async fn success_with_valid_password() {
    let app = spawn_app().await;

    let res = app
        .login(&app.test_user.username, &app.test_user.password)
        .await;

    // check the response code is 200
    assert_eq!(res.status().as_u16(), 200);

    // make sure the token string is inside the response json
    let json: serde_json::Value = res.json().await.unwrap();
    assert!(json.as_object().unwrap().get("token").is_some());
}

#[tokio::test]
async fn failure_with_bad_password() {
    let app = spawn_app().await;

    let res = app
        .login(&app.test_user.username, "not_valid_password")
        .await;

    assert_eq!(res.status().as_u16(), 401);
}

#[tokio::test]
async fn failure_with_non_exist_username() {
    let app = spawn_app().await;

    let res = app.login("not_exist", "password").await;

    assert_eq!(res.status().as_u16(), 401);
}
