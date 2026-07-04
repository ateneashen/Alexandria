# Alexandria

Indexador local de activos digitales escrito en Rust. Escanea directorios, extrae metadatos de video, audio, PDF y ZIP, detecta grupos por patrones de nombre y los sirve a través de una interfaz web ligera.

## Características

- **CLI portable**: escanea carpetas, levanta servidor y consulta estadísticas.
- **Metadatos de video**: duración, resolución, codecs, pistas de audio y subtítulos (vía `ffprobe`).
- **Metadatos de audio**: duración, codec y tags (título, artista, álbum, género, fecha) vía `ffprobe`.
- **Documentos y archivos comprimidos**: extracción de páginas e información de PDFs (`lopdf`) y listado de contenido de archivos ZIP.
- **Agrupación inteligente**: detecta automáticamente series, películas (incluyendo versiones/remakes) y colecciones por prefijo.
- **Notas y etiquetas**: añade notas históricas y tags a cualquier archivo; gestión desde CLI y web.
- **Reorganización física de archivos (beta)**: mueve/renombra archivos según plantillas basadas en metadatos, grupos, fecha o tags; con dry-run, backup de BD, verificación de checksums, rollback y **estimación de espacio en disco**.
- **Base de datos SQLite embebida**: sin instalación externa.
- **Interfaz web vanilla renovada**: navegación por sidebar, dashboard con gráficos, listado de archivos con filtros avanzados, modal de detalle con pestañas, grupos visuales y wizard de reorganización; todo embebido en el binario.
- **Single binary**: copia y ejecuta desde cualquier carpeta.
- **Fallback robusto**: funciona solo con metadatos del sistema de archivos si `ffprobe` no está instalado.

## Requisitos

- [Rust](https://www.rust-lang.org/) 1.75 o superior.
- (Opcional) `ffprobe` en el PATH para metadatos avanzados de video y audio.

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

Abre http://127.0.0.1:3000 en tu navegador. Verás una interfaz con sidebar, dashboard, listado de archivos, grupos y el wizard de reorganización.

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

### Añadir una nota a un archivo

```bash
alexandria note "C:/Users/Admin/Videos/movie.mp4" --content "Versión extendida"
```

### Gestionar etiquetas de un archivo

```bash
alexandria tag "C:/Users/Admin/Videos/movie.mp4" --add favoritos --add pendiente
alexandria tag "C:/Users/Admin/Videos/movie.mp4" --remove pendiente
```

### Reorganizar archivos físicamente

> ⚠️ Función de riesgo. Siempre prueba con `--dry-run` y ten un backup externo antes de usar `apply`.

Generar un plan de reorganización:

```bash
alexandria reorg plan --strategy by-type \
  --template "{file_type}/{name}.{ext}" \
  --target-root "D:/Organizado" \
  --file-type video \
  --dry-run
```

Aplicar un plan (pide confirmación; usa `--yes` para saltarla):

```bash
alexandria reorg apply --job-id 1
```

Hacer rollback de un plan aplicado:

```bash
alexandria reorg rollback --job-id 1
```

Estrategias disponibles: `by-type`, `by-group`, `by-date`, `by-tag`. Tokens de plantilla: `{file_type}`, `{extension}`, `{name}`, `{ext}`, `{group_name}`, `{group_kind}`, `{year}`, `{month}`, `{day}`, `{tag}`.

Antes de aplicar, Alexandria muestra:
- Capacidad total, espacio libre y usado del disco de destino.
- Tamaño total de archivos seleccionados.
- Espacio adicional requerido (0 si todos los movimientos son atómicos en el mismo volumen).
- Consejo del sistema y advertencias si hay poco espacio o archivos grandes.

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
- `GET /api/stats` — estadísticas (totales, por tipo de archivo, grupos, último escaneo).
- `GET /api/stats/by-type` — conteo de archivos por tipo.
- `GET /api/files` — listar archivos con filtros (`name`, `extension`, `file_type`, `min_size`, `max_size`, `has_subtitles`, `group_id`, `modified_after`, `modified_before`, `sort_by`, `sort_order`, `limit`, `offset`).
- `GET /api/files/count` — conteo de archivos según los mismos filtros.
- `GET /api/files/:id` — detalle de un archivo.
- `GET /api/files/:id/notes` — historial de notas del archivo.
- `POST /api/files/:id/notes` — añadir/editar nota principal e insertar en el historial.
- `DELETE /api/notes/:id` — eliminar una nota del historial.
- `GET /api/files/:id/tags` — etiquetas del archivo.
- `POST /api/files/:id/tags` — asignar etiqueta (la crea si no existe).
- `DELETE /api/files/:id/tags/:tag_id` — desasignar etiqueta.
- `GET /api/file-types` — tipos de archivo indexados.
- `GET /api/extensions` — extensiones indexadas.
- `GET /api/scan-jobs` — últimos trabajos de escaneo.
- `GET /api/system/storage` — información de discos del sistema (capacidad, libre, usado).
- `GET /api/reorganize/strategies` — estrategias y tokens de reorganización.
- `POST /api/reorganize/plan` — generar plan de reorganización.
- `GET /api/reorganize/jobs` — listar trabajos de reorganización.
- `GET /api/reorganize/jobs/:id` — detalle de un trabajo.
- `POST /api/reorganize/jobs/:id/apply` — aplicar plan.
- `POST /api/reorganize/jobs/:id/rollback` — revertir plan.
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
  reorganizer/     # Planificación/ejecución de reorganización física
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
- Validación a gran escala de la reorganización física (beta en curso en `Z:\AlexandriaProjectBeta`).
- Soporte para más formatos de archivo (RAR, 7z, imágenes, etc.).
- Pruebas de carga y estabilidad en discos grandes.

## Licencia

MIT. Ver [LICENSE](LICENSE).
