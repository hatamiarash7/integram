use actix_web::{get, post, App, HttpResponse, HttpServer, middleware, Responder, web};
use serde::{Deserialize};
use telegram_gitlab::{find_chat_by_id, find_webhook_by_webhook_url};

#[derive(Debug, Deserialize)]
struct GitlabEvent {
    object_kind: String,
    event_name: String,
    before: String,
    after: String,
    r#ref: String,
    checkout_sha: String,
    message: Option<String>,
    user_id: u32,
    user_name: String,
    user_username: String,
    user_email: String,
    user_avatar: String,
    project_id: u32,
    project: Project,
    repository: Repository,
    commits: Vec<Commit>,
    total_commits_count: u32,
}

#[derive(Debug, Deserialize)]
struct Project {
    id: u32,
    name: String,
    description: String,
    web_url: String,
    avatar_url: Option<String>,
    git_ssh_url: String,
    git_http_url: String,
    namespace: String,
    visibility_level: u32,
    path_with_namespace: String,
    default_branch: String,
    ci_config_path: Option<String>,
    homepage: String,
    url: String,
    ssh_url: String,
    http_url: String,
}

#[derive(Debug, Deserialize)]
struct Repository {
    name: String,
    url: String,
    description: String,
    homepage: String,
}

#[derive(Debug, Deserialize)]
struct Commit {
    id: String,
    message: String,
    timestamp: String,
    url: String,
}

pub async fn run_http_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(health)
            .service(handle_gitlab_webhook)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

#[get("/health")]
async fn health() -> impl Responder {
    log::info!("Health check");
    "I'm ok"
}

#[derive(Deserialize)]
struct GitlabWebhook {
    webhook_url: String,
}

#[post("/gitlab/{webhook_url}")]
async fn handle_gitlab_webhook(gitlab_webhook: web::Path<GitlabWebhook>, gitlab_event: web::Json<GitlabEvent>) -> impl Responder {
    let branch_ref = &gitlab_event.r#ref;
    let branch_name = branch_ref.split('/').last().unwrap();
    let project_name = &gitlab_event.project.name;
    let commit_message = &gitlab_event.commits[0].message;
    let commit_url = &gitlab_event.commits[0].url;

    let webhook_url = &gitlab_webhook.webhook_url;
    log::info!("webhook_url: {}", webhook_url);
    let webhook = find_webhook_by_webhook_url(webhook_url);

    if webhook.is_none() {
        log::error!("Webhook not found");
        return HttpResponse::NotFound();
    }
    let webhook = webhook.unwrap();

    // log chat_id
    log::info!("Webhook: {}", webhook.webhook_url);
    let chat_id = webhook.chat_id.expect("Chat id must be set");
    log::info!("Chat id: {}", chat_id);

    let chat = find_chat_by_id(webhook.chat_id.expect("Chat id must be set"));

    if chat.is_none() {
        log::error!("Chat not found");
        return HttpResponse::NotFound();
    }
    let chat = chat.unwrap();

    let message = format!("{}: {} - {} - {}", project_name, branch_name, commit_message, commit_url);

    crate::telegram_bot::send_message(chat.telegram_id.parse::<i64>().expect("CHAT_ID must be an integer"), message).await.unwrap();

    log::info!("bot sent message");
    HttpResponse::Ok() // <- send response
}

