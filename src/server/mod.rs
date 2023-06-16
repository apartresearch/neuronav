use std::sync::Arc;

use actix_web::{
    get, http::header::ContentType, rt, web, App, HttpResponse, HttpServer, Responder,
};

use crate::Neuronav;

#[get("/api/{model}/{service}/{layer_index}/{neuron_index}")]
async fn index(
    neuronav: web::Data<&Neuronav>,
    indices: web::Path<(String, String, u32, u32)>,
) -> impl Responder {
    let (model_name, service_name, layer_index, neuron_index) = indices.into_inner();

    match neuronav.handle_request(model_name, service_name, layer_index, neuron_index) {
        Ok(page) => HttpResponse::Ok().content_type(ContentType::json()).body(
            serde_json::to_string(&page)
                .expect("Failed to serialize page to JSON. This should always be possible."),
        ),
        Err(error) => HttpResponse::ServiceUnavailable().body(format!("{error}")),
    }
}

pub fn start_server(neuronav: Arc<Neuronav>) -> std::io::Result<()> {
    rt::System::new().block_on(
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(Arc::clone(&neuronav)))
                .service(index)
        })
        .bind(("127.0.0.1", 8080))?
        .run(),
    )
}
