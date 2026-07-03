# Alexandria

> Proyecto generado en: `C:\AI\2026 Projects\KIMI\AlexandriaProject`

## 📖 Visión General
**Alexandria** es una aplicación escrita en Rust diseñada para escanear sistemas de archivos locales, extraer metadatos de archivos (foco inicial en video) y crear una base de datos centralizada accesible vía navegador.

El objetivo final es convertirse en un gestor universal de activos digitales con capacidades de:
1. **Ingesta masiva:** Escaneo recursivo de discos duros.
2. **Base de datos indexada:** Almacenamiento eficiente de metadatos y contenido en SQLite.
3. **Interfaz Web:** Visualización, filtrado avanzado y gestión mediante HTML/JS.
4. **Inteligencia de agrupación:** Detección automática de patrones en nombres (series, versiones de películas).
5. **Extensibilidad:** Arquitectura diseñada para soportar nuevos tipos de archivos sin refactorizar el núcleo.

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

### 4. Extensibilidad (Fase Futura)
- Soporte para:
  - **Documentos:** PDF, DOCX (extraer título, autor, palabras).
  - **Backups:** ZIP, RAR, TAR (listar contenido).
  - **Modelos IA:** Weights, checkpoints (extraer nombre de modelo, tamaño).
- Diseño de módulos para nuevos extractores de metadatos.

## 🏗️ Arquitectura Técnica

### Stack Tecnológico
- **Lenguaje:** Rust (rendimiento, seguridad y manejo de archivos pesado).
- **Base de Datos Local:** SQLite embebido vía `sqlx`.
- **Servidor Web:** Axum (ligero, moderno, basado en Tokio).
- **Frontend:** HTML5 + CSS3 + JavaScript vanilla, embebido en el binario.
- **Herramientas de Análisis:**
  - `ffprobe` opcional para inspección profunda de video.
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
│   └── 001_initial.sql
├── src/
│   ├── main.rs               # Punto de entrada y lógica de inicialización
│   ├── cli.rs                # Argumentos de línea de comandos
│   ├── config.rs             # Configuración y resolución de paths
│   ├── db.rs                 # Gestión de SQLite (migraciones, CRUD)
│   ├── models.rs             # Estructuras de datos compartidas
│   ├── error.rs              # Tipos de error de la aplicación
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

### Fase 2: Escáner y Extractores (Video)
- [ ] Implementar escaneo recursivo de archivos.
- [ ] Implementar extractor de metadatos para videos.
- [ ] Integrar con `ffprobe` de forma opcional.
- [ ] Guardar metadatos en SQLite.

### Fase 3: Servidor Web e Interfaz
- [ ] Servidor Axum que sirva la API y el frontend estático.
- [ ] Interfaz HTML/JS para visualización y filtrado.
- [ ] API REST para operaciones (añadir nota, filtrar).

### Fase 4: Agrupación y Patrones
- [ ] Detectar series y películas duplicadas por nombre.
- [ ] Crear tabla de "grupos" en la BD.

### Fase 5: Extensibilidad y Refinamiento
- [ ] Sistema de módulos para otros formatos (PDF, ZIP).
- [ ] Mejorar UI/UX.
- [ ] Pruebas de carga y estabilidad en discos grandes.

## ⚠️ Consideraciones de Seguridad y Robustez

- **Manejo de errores:** Logs detallados por cada archivo fallido.
- **Recursos:** Evitar bloquear el sistema al escanear terabytes; usar async y límites de concurrencia.
- **Privacidad:** Todo se procesa localmente, sin enviar datos a servidores externos.
- **Portabilidad:** Single binary, paths relativos, SQLite embebido, `ffprobe` opcional.

---

Documentación generada para el proyecto **Alexandria**.
Última actualización: 2026-07-03
