// #[cfg(test)]
// mod tests {
//     use axum::{
//         body::Body,
//         http::{self, Request, StatusCode},
//     };
//     use tower::ServiceExt; // Needed for .oneshot()
//     use blog::{app_with_db, PaginatedResponse, PostSummary}; 

//     #[sqlx::test]
//     async fn test_create_and_list_posts(pool: sqlx::PgPool) {
//         // 1. Initialize your actual Axum Router using the test pool
//         let app = app_with_db(pool).await;

//         // 2. Prepare mock data for a new post
//         let new_post_payload = serde_json::json!({
//             "title": "My First Test Post",
//             "content": "This is content generated inside a test sandbox.",
//             "genre": "technology"
//         });

//         // 3. Construct a simulated HTTP POST request
//         // (Note: Your router expects a Bearer token matching BLOG_SYNC_TOKEN)
//         std::env::set_var("BLOG_SYNC_TOKEN", "super-secret-test-token");
        
//         let request = Request::builder()
//             .method(http::Method::POST)
//             .uri("/posts/my-first-post")
//             .header(http::header::CONTENT_TYPE, "application/json")
//             .header(http::header::AUTHORIZATION, "Bearer super-secret-test-token")
//             .body(Body::from(serde_json::to_vec(&new_post_payload).unwrap()))
//             .unwrap();

//         // 4. Send the request to your application instance
//         let response = app.clone().oneshot(request).await.unwrap();

//         // 5. Assert the post was successfully created
//         assert_eq!(response.status(), StatusCode::CREATED);

//         // 6. Verify the post now appears when querying the GET /posts endpoint
//         let get_request = Request::builder()
//             .method(http::Method::GET)
//             .uri("/posts?genre=technology")
//             .body(Body::empty())
//             .unwrap();

//         let get_response = app.oneshot(get_request).await.unwrap();
//         assert_eq!(get_response.status(), StatusCode::OK);

//         // 7. Parse and inspect the JSON response body
//         let bytes = axum::body::to_bytes(get_response.into_body(), usize::MAX)
//             .await
//             .unwrap();
        
//         let body: PaginatedResponse<PostSummary> = serde_json::from_slice(&bytes).unwrap();
        
//         // Assertions on the data payload
//         assert_eq!(body.total, 1);
//         assert_eq!(body.items[0].title, "My First Test Post");
//     }
// }
