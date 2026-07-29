use std::path::PathBuf;

pub fn dir_datos() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from("data")
    }
    #[cfg(not(debug_assertions))]
    {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("JUAN-VIVI")
    }
}

pub fn ruta_archivo(nombre: &str) -> PathBuf {
    dir_datos().join(nombre)
}
