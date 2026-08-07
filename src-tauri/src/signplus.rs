use anyhow::anyhow;
use chrono::Local;
use reqwest::Client;

const URL_BASE: &str = "http://192.168.1.177";
const URL_API: &str = "http://192.168.1.177/app/escrituras_publicas/api/";
const USUARIO: &str = "JRIOS";
const PASSWORD: &str = "Cpina2028";

const MESES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio",
    "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre",
];

fn generar_cuerpo_cert(tipo: &str, dia: u32, mes: u32, anio: u32) -> String {
    let mes_nombre = MESES
        .get((mes as usize).saturating_sub(1))
        .copied()
        .unwrap_or("?");
    format!(
        "Certifico que el presente documento electrónico es copia fiel e íntegra de {} \
         otorgado el {} de {} de {} reproducido en las siguientes páginas.",
        tipo, dia, mes_nombre, anio
    )
}

async fn login() -> anyhow::Result<(Client, String)> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .cookie_store(true)
        .build()?;

    let login_url = format!("{}/login.php", URL_BASE);
    let params = [
        ("username", USUARIO),
        ("password", PASSWORD),
        ("secondLogin", "s"),
    ];

    let resp: reqwest::Response = client
        .post(&login_url)
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Login HTTP {}", resp.status()));
    }

    let html: String = resp.text().await?;

    let re = regex::Regex::new(r"localStorage\.setItem\('token',\s*'([^']+)'\)")?;
    let token = re
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("No se encontró el token JWT en la respuesta de login"))?;

    Ok((client, token))
}

pub async fn enviar_formulario(
    ruta_pdf: &std::path::Path,
    numero: &str,
    anho: &str,
    tipo_contrato: &str,
    fecha_dia: u32,
    fecha_mes: u32,
    fecha_anio: u32,
) -> anyhow::Result<()> {
    let (client, token) = login().await?;

    let hoy = Local::now().format("%Y-%m-%d").to_string();
    let cuerpo_cert = generar_cuerpo_cert(tipo_contrato, fecha_dia, fecha_mes, fecha_anio);

    let anho_int: i64 = anho.parse().unwrap_or(0);

    let datos = serde_json::json!({
        "ot": "",
        "repertorio": numero,
        "anho": anho_int,
        "tipo_contrato": tipo_contrato,
        "email": "",
        "fecha_escritura": format!("{:04}-{:02}-{:02}", fecha_anio, fecha_mes, fecha_dia),
        "monto": "",
        "moneda": "PESOS",
        "voltear": false,
        "fecha_emision": hoy,
        "matricera": "",
        "banco": "",
        "modificar_certificacion": false,
        "cuerpo_cert": cuerpo_cert,
        "existe_pdf": "nop",
        "url_pdf_existente": "",
        "lista_comparecientes": []
    });

    let datos_str = datos.to_string();

    let pdf_bytes = std::fs::read(ruta_pdf)?;
    let nombre_pdf = ruta_pdf
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let pdf_part = reqwest::multipart::Part::bytes(pdf_bytes)
        .file_name(nombre_pdf)
        .mime_str("application/pdf")?;

    let form = reqwest::multipart::Form::new()
        .text("accion", "guardar_copia_compulsa")
        .text("datos", datos_str)
        .part("file_principal", pdf_part);

    let resp = client
        .post(URL_API)
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Upload HTTP {}", resp.status()));
    }

    let resultado: serde_json::Value = resp.json().await?;

    if !resultado["estado"].as_bool().unwrap_or(false) {
        let mensaje = resultado["mensaje"]
            .as_str()
            .unwrap_or("Error desconocido")
            .to_string();
        return Err(anyhow!("SIGN+: {}", mensaje));
    }

    Ok(())
}
