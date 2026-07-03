# Alexandria

Indexador local de activos digitales escrito en Rust. Escanea directorios, extrae metadatos (foco inicial en video), detecta grupos por patrones de nombre y los sirve a través de una interfaz web ligera.

## Características

- **CLI portable**: escanea carpetas, levanta servidor y consulta estadísticas.
- **Metadatos de video**: duración, resolución, codecs, pistas de audio y subtítulos (via `ffprobe` si está disponible).
- **Agrupación inteligente**: detecta automáticamente series, películas (incluyendo versiones/remakes) y colecciones por prefijo.
- **Base de datos SQLite embebida**: sin instalación externa.
- **Interfaz web vanilla**: embebida en el binario, lista para usar.
- **Single binary**: copia y ejecuta desde cualquier carpeta.
- **Fallback robusto**: funciona solo con metadatos del sistema de archivos si `ffprobe` no está instalado.

## Requisitos

- [Rust](https://www.rust-lang.org/) 1.75 o superior.
- (Opcional) `ffprobe` en el PATH para metadatos avanzados de video.

## Instalación

```bash
git clone https://github.com/ateneashen/alexandria.git
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

### Listar grupos detectados

```bash
alexandria groups
```

Filtrar por tipo:

```bash
alexandria groups --kind series
```

### Recalcular grupos

Si ya tienes archivos indexados y quieres aplicar nuevas reglas de agrupación:

```bash
alexandria regroup
```

## Agrupación inteligente

Alexandria clasifica automáticamente los archivos en grupos:

| Tipo | Patrones reconocidos | Ejemplo |
|------|---------------------|---------|
| **series** | `S01E02`, `1x02`, `S1E2` | `Show.Name.S01E02.mp4` |
| **movie** | Año (1900-2099) + título | `Movie.Name.2024.1080p.mp4` |
| **collection** | Prefijo común de palabras | `Some Random File.txt` |

Las películas con el mismo título y año pero diferentes versiones (Director's Cut, Extended, Remux, etc.) se agrupan juntas.

## API REST

- `GET /api/health` — estado del servidor.
- `GET /api/stats` — estadísticas.
- `GET /api/files` — listar archivos con filtros (`name`, `extension`, `file_type`, `min_size`, `max_size`, `has_subtitles`, `group_id`).
- `GET /api/files/:id` — detalle de un archivo.
- `POST /api/files/:id/notes` — añadir/editar nota.
- `GET /api/groups` — listar grupos (`?kind=series`).
- `GET /api/groups/:id` — detalle de un grupo.
- `GET /api/groups/:id/files` — archivos de un grupo.

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
  groups/          # Motor de agrupación por patrones
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

Próximas fases:
- Soporte para PDF, ZIP y audio.
- Mejoras de UI/UX en el frontend.

## Licencia

MIT. Ver [LICENSE](LICENSE).
