# Alexandria

Indexador local de activos digitales escrito en Rust. Escanea directorios, extrae metadatos (foco inicial en video) y los sirve a través de una interfaz web ligera.

## Características (MVP)

- **CLI portable**: escanea carpetas, levanta servidor y consulta estadísticas.
- **Metadatos de video**: duración, resolución, codecs, pistas de audio y subtítulos (via `ffprobe` si está disponible).
- **Base de datos SQLite embebida**: sin instalación externa.
- **Interfaz web vanilla**: embebida en el binario, lista para usar.
- **Single binary**: copia y ejecuta desde cualquier carpeta.
- **Fallback robusto**: funciona solo con metadatos del sistema de archivos si `ffprobe` no está instalado.

## Requisitos

- [Rust](https://www.rust-lang.org/) 1.75 o superior.
- (Opcional) `ffprobe` en el PATH para metadatos avanzados de video.

## Instalación

```bash
git clone https://github.com/tu-usuario/alexandria.git
cd alexandria
cargo build --release
```

El ejecutable estará en `target/release/alexandria` (o en `$CARGO_TARGET_DIR/release/alexandria` si usas `CARGO_TARGET_DIR`).

### Nota para Windows con espacios en la ruta del proyecto

Si clonas el proyecto en una ruta que contiene espacios (por ejemplo `C:\Mis Proyectos\alexandria`), la compilación de `libsqlite3-sys` puede fallar con `LNK1104`. Para evitarlo, define un directorio de build sin espacios:

```powershell
$env:CARGO_TARGET_DIR = "C:\alexandria-target"
cargo build --release
```

O en Git Bash:

```bash
export CARGO_TARGET_DIR=/c/alexandria-target
cargo build --release
```

## Uso

### Escanear un directorio

```bash
alexandria scan "C:/Users/Admin/Videos"
```

Opciones:
- `--concurrency <n>`: tareas concurrentes (default: 4).
- `--force`: re-escanear archivos ya indexados.
- `--data-dir <path>`: directorio donde guardar la base de datos y logs.

### Iniciar el servidor web

```bash
alexandria serve
```

Abre http://127.0.0.1:3000 en tu navegador.

### Ver información

```bash
alexandria info
```

## Portabilidad

Alexandria intenta guardar sus datos en una carpeta `.alexandria` junto al ejecutable. Si no tiene permisos de escritura, usa el directorio de datos del usuario:

- Windows: `%APPDATA%/Alexandria`
- Linux/Mac: `~/.local/share/alexandria`

Puedes forzar el directorio con `--data-dir`.

## Estructura del proyecto

```text
src/
  main.rs          # Punto de entrada
  cli.rs           # Argumentos de línea de comandos
  config.rs        # Configuración y paths
  db.rs            # Capa SQLite
  models.rs        # Estructuras de datos
  error.rs         # Tipos de error
  scanner/         # Escaneo de directorios
  extractors/      # Extracción de metadatos
  server/          # API REST + frontend
schemas/           # Migraciones SQL
docs/              # Documentación de progreso
tests/             # Tests de integración
```

## Tests

```bash
cargo test
```

## Roadmap

Ver [AlexandriaProject.MD](AlexandriaProject.MD) para la hoja de ruta completa.

## Licencia

MIT. Ver [LICENSE](LICENSE).
