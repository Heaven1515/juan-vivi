use serde::Serialize;

const URL_LICENCIA: &str =
    "https://api.github.com/repos/Heaven1515/juan-vivi/contents/licencia.json";

#[derive(Debug, Serialize)]
pub struct EstadoLicencia {
    pub activo: bool,
    pub mensaje: String,
}

impl EstadoLicencia {
    fn permisivo() -> Self {
        EstadoLicencia { activo: true, mensaje: String::new() }
    }
}

#[tauri::command]
pub async fn verificar_licencia() -> EstadoLicencia {
    let resultado = intentar_verificar().await;
    resultado.unwrap_or_else(|_| EstadoLicencia::permisivo())
}

async fn intentar_verificar() -> anyhow::Result<EstadoLicencia> {
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;

    let resp = cliente
        .get(URL_LICENCIA)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "juan-vivi")
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    let content_b64 = json["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("sin campo content"))?;

    // GitHub incluye saltos de línea en el base64
    let limpio: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &limpio)?;
    let texto = String::from_utf8(bytes)?;
    let datos: serde_json::Value = serde_json::from_str(&texto)?;

    let activo = datos["activo"].as_bool().unwrap_or(true);
    let mensaje = datos["mensaje"].as_str().unwrap_or("").to_string();

    Ok(EstadoLicencia { activo, mensaje })
}
