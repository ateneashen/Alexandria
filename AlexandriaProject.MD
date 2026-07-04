# Alexandria

> Proyecto generado en: `C:\AI\2026 Projects\KIMI\AlexandriaProject`

## 📖 Visión General
**Alexandria** es una aplicación escrita en Rust diseñada para escanear sistemas de archivos locales, extraer metadatos de archivos (video, audio, PDF, ZIP y más en el futuro) y crear una base de datos centralizada accesible vía navegador.

El objetivo final es convertirse en un gestor universal de activos digitales con capacidades de:
1. **Ingesta masiva:** Escaneo recursivo de discos duros.
2. **Base de datos indexada:** Almacenamiento eficiente de metadatos y contenido en SQLite.
3. **Interfaz Web:** Visualización, filtrado avanzado y gestión mediante HTML/JS.
4. **Inteligencia de agrupación:** Detección automática de patrones en nombres (series, versiones de películas).
5. **Reorganización física segura:** Mover/renombrar archivos en disco según metadatos, grupos, fecha o tags; con dry-run, backup de BD, verificación de integridad y rollback.
6. **Extensibilidad:** Arquitectura diseñada para soportar nuevos tipos de archivos y operaciones sin refactorizar el núcleo.

## 🎯 Objetivos Principales

### 1. Análisis de Metadatos (Fase Video)
- Detectar formato de video (MKV, MP4, AVI, MOV, WEBM, etc.).
- Extraer propiedades: duración, resolución, codec, tamaño, fecha de modificación.
- Identificar contenido especial: subtítulos, audio multilenguaje.
- Uso opcional de `ffprobe` para metadatos profundos; fallback a metadatos del sistema de archivos si no está disponible.

### 2. Inteligencia de Agrupación
- Análisis de patrones parciales en nombres de archivo.
- Asociación automática de:
  - Series (capítulos relacionados).
  - Versiones de películas (remakes, directoriales, BluRay vs Remux).
  - Colecciones temáticas.

### 3. Gestión de Datos y Notas
- Crear notas personalizadas por usuario para ficheros o grupos filtrados.
- Sistema de búsqueda semántica (por nombre, tamaño, etiquetas).

### 4. Extensibilidad
- Arquitectura de extractores plug-and-play bajo `src/extractors/`.
- Formatos ya soportados: video, audio, PDF, ZIP.
- Formatos futuros: DOCX, RAR, 7z, imágenes, modelos IA, etc.

## 🏗️ Arquitectura Técnica

### Stack Tecnológico
- **Lenguaje:** Rust (rendimiento, seguridad y manejo de archivos pesado).
- **Base de Datos Local:** SQLite embebido vía `sqlx`.
- **Servidor Web:** Axum (ligero, moderno, basado en Tokio).
- **Frontend:** HTML5 + CSS3 + JavaScript vanilla, embebido en el binario.
- **Herramientas de Análisis:**
  - `ffprobe` opcional para inspección profunda de video y audio.
  - `lopdf` para metadatos de PDF.
  - `zip` para listado de contenidos ZIP.
  - Extracción básica del sistema de archivos como fallback.

### Estructura del Proyecto
```text
AlexandriaProject/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── .gitignore
├── docs/                     # Documentación técnica y de progreso
│   ├── progreso.md
│   ├── decisiones.md
│   └── errores.md
├── schemas/                  # Definición de esquemas de la BD (SQL)
│   ├── 1_initial.sql
│   └── 2_add_groups.sql
├── src/
│   ├── main.rs               # Punto de entrada y lógica de inicialización
│   ├── cli.rs                # Argumentos de línea de comandos
│   ├── config.rs             # Configuración y resolución de paths
│   ├── db.rs                 # Gestión de SQLite (migraciones, CRUD)
│   ├── models.rs             # Estructuras de datos compartidas
│   ├── error.rs              # Tipos de error de la aplicación
│   ├── groups/               # Motor de agrupación por patrones
│   ├── scanner/              # Motor de escaneo de directorios
│   ├── extractors/           # Lógica para extraer datos por tipo
│   └── server/               # Servidor HTTP, API REST y frontend estático
└── tests/                    # Tests de integración
```

## 📋 Requisitos Funcionales

1. **Escaneo:** Opción para escanear un disco entero o carpetas específicas.
2. **Filtros Avanzados:**
   - Búsqueda por nombre (parcial).
   - Filtro por tamaño (rango min/max).
   - Filtro por propiedades técnicas (resolución, subtítulos presentes).
   - Filtro por "familia" (grupos detectados por patrones).
3. **Interfaz Web:**
   - Dashboard visual.
   - Panel de detalles del archivo con metadatos extraídos.
   - Editor de notas contextual.
4. **Logs y Robustez:**
   - Archivo `logs/alexandria.log` detallado con timestamps.
   - Manejo de errores por archivo: se loguea pero no se detiene el escaneo.
   - Límites de concurrencia para no bloquear el sistema.

## 🚀 Hoja de Ruta (Roadmap)

### Fase 1: Núcleo y Estructura (MVP)
- [x] Inicializar repositorio y estructura de carpetas.
- [x] Configurar `Cargo.toml` con dependencias base (tokio, axum, sqlx, clap).
- [x] Implementar sistema de logs con `tracing`.
- [x] Definir esquema de la base de datos SQLite.

### Fase 2: Más Formatos (v0.2.0)
- [x] Implementar extractor de metadatos para audio (vía `ffprobe`).
- [x] Implementar extractor de metadatos para PDF (`lopdf`).
- [x] Implementar extractor de contenido para ZIP.
- [x] Detectar tipo de archivo (`audio`, `pdf`, `archive`) en el escáner.
- [x] Guardar metadatos extendidos en `extra_json`.

### Fase 3: Agrupación y Patrones
- [x] Implementar agrupación por patrones: series, películas y colecciones.
- [x] Crear tabla de "groups" en la BD y asignar `group_id` a archivos.
- [x] API REST para grupos (`/api/groups`).
- [x] CLI `groups` y `regroup`.

### Fase 4: Base de Datos y Notas
- [x] Tablas para notes y user_tags.
- [x] Funcionalidad para añadir notas desde CLI o Web.

### Fase 5: Servidor Web e Interfaz
- [x] Servidor Axum que sirva la API y el frontend estático.
- [x] Interfaz HTML/JS para visualización, dashboard, grupos y filtrado avanzado.
- [x] API REST para operaciones (notas, etiquetas, filtrar, ordenar).

### Fase de Reorganización Física (v0.4.0)
- [x] Diseño y planificación de seguridad (dry-run, backup, checksums, rollback).
- [x] Tablas `reorg_jobs` y `reorg_operations`.
- [x] Motor de plantillas con tokens (`{file_type}`, `{group_name}`, `{year}`, `{tag}`, etc.).
- [x] Planificador con detección de colisiones y destinos peligrosos.
- [x] Ejecutor con movimientos atómicos y copy+verify entre volúmenes.
- [x] CLI `reorg plan/list/status/apply/rollback`.
- [x] API REST `/api/reorganize/*`.
- [x] Pestaña "Reorganizar" en el frontend.
- [x] Beta funcional en `Z:\AlexandriaProjectBeta`.

### Fase 6: Extensibilidad y Refinamiento
- [x] Sistema de módulos para otros formatos (PDF, ZIP, audio).
- [x] Mejorar UI/UX (dashboard, pestañas, badges, responsividad).
- [ ] Validar reorganización física a gran escala.
- [ ] Pruebas de carga y estabilidad en discos grandes.
- [ ] Soporte para más formatos (RAR, 7z, imágenes, modelos IA).

## ⚠️ Consideraciones de Seguridad y Robustez

- **Manejo de errores:** Logs detallados por cada archivo fallido.
- **Recursos:** Evitar bloquear el sistema al escanear terabytes; usar async y límites de concurrencia.
- **Privacidad:** Todo se procesa localmente, sin enviar datos a servidores externos.
- **Portabilidad:** Single binary, paths relativos, SQLite embebido, `ffprobe` opcional.

---

Documentación generada para el proyecto **Alexandria**.
Última actualización: 2026-07-03
