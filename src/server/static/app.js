/**
 * Alexandria Frontend — app.js
 * ==============================
 * Aplicación vanilla que sirve de interfaz de usuario para el indexador
 * local Alexandria. No usa frameworks ni dependencias de red.
 *
 * Estructura didáctica:
 * 1. Estado global
 * 2. Helpers de API
 * 3. Utilidades de presentación
 * 4. Componentes reutilizables
 * 5. Vistas (Dashboard, Archivos, Grupos, Reorganizar)
 * 6. Detalle de archivo (modal con pestañas)
 * 7. Inicialización y eventos
 */

// ============================================================
// 1. ESTADO GLOBAL
// ============================================================

/** Vista activa en la navegación principal. */
let currentView = 'dashboard';

/** Configuración de paginación para la lista de archivos. */
let filePage = {
    size: 25,
    offset: 0,
    count: 0,
};

/** Filtros actuales aplicados a la lista de archivos. */
let fileFilters = {
    name: null,
    extension: null,
    file_type: null,
    min_size: null,
    max_size: null,
    has_subtitles: null,
    group_id: null,
    modified_after: null,
    modified_before: null,
    sort_by: 'name',
    sort_order: 'asc',
};

/** ID del archivo seleccionado en el modal de detalle. */
let selectedFileId = null;

/** Mapa id -> nombre de grupo para mostrar nombres legibles. */
let groupsMap = new Map();

/** Cache de estrategias de reorganización devueltas por el backend. */
let reorgStrategies = [];

/** ID del job de reorganización activo. */
let currentReorgJobId = null;

/** Estrategia seleccionada en el wizard. */
let selectedReorgStrategy = null;

/** Temporizador para debounce del buscador. */
let searchDebounceTimer = null;

/** Temporizador de refresco automático de escaneos en curso. */
let scanPollTimer = null;

// ============================================================
// 2. HELPERS DE API
// ============================================================

/**
 * Realiza una petición GET a la API y devuelve JSON.
 * Muestra un toast si falla la conexión.
 */
async function apiGet(path) {
    const res = await fetch(path);
    if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new Error(`GET ${path} → ${res.status}: ${text}`);
    }
    return res.json();
}

/**
 * Realiza una petición POST con body JSON.
 */
async function apiPost(path, body) {
    const res = await fetch(path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
        throw new Error(data.error || `POST ${path} → ${res.status}`);
    }
    return data;
}

/**
 * Realiza una petición DELETE.
 */
async function apiDelete(path) {
    const res = await fetch(path, { method: 'DELETE' });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
        throw new Error(data.error || `DELETE ${path} → ${res.status}`);
    }
    return data;
}

// ============================================================
// 3. UTILIDADES DE PRESENTACIÓN
// ============================================================

/** Formatea bytes a unidades humanas (B, KB, MB...). */
function formatBytes(bytes) {
    if (bytes === 0 || bytes == null) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/** Formatea segundos como "Xh Ym Zs". */
function formatDuration(seconds) {
    if (seconds == null) return '-';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${h}h ${m}m ${s}s`;
}

/** Formatea una fecha ISO a texto local. */
function formatDate(iso) {
    if (!iso) return 'Nunca';
    return new Date(iso).toLocaleString();
}

/** Escapa HTML para evitar XSS. */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/** Devuelve un badge con color según el tipo de archivo. */
function typeBadge(fileType) {
    const type = (fileType || 'unknown').toLowerCase();
    const icons = { video: '🎬', audio: '🎵', pdf: '📄', archive: '🗜️', unknown: '❓' };
    return `<span class="badge type-${type}">${icons[type] || ''} ${escapeHtml(type)}</span>`;
}

/** Devuelve el nombre de un grupo a partir de su id. */
function groupName(id) {
    if (id == null) return '-';
    return groupsMap.get(String(id)) || `Grupo ${id}`;
}

/** Devuelve un icono según el tipo de grupo. */
function groupIcon(kind) {
    switch (kind) {
        case 'series': return '📺';
        case 'movie': return '🎬';
        case 'collection': return '📁';
        default: return '📦';
    }
}

/** Muestra un toast en la esquina inferior derecha. */
function showToast(message, type = 'info') {
    const container = document.getElementById('toast-container');
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    const icons = { success: '✅', error: '❌', warning: '⚠️', info: 'ℹ️' };
    toast.innerHTML = `
        <span>${icons[type] || 'ℹ️'}</span>
        <span class="toast-message">${escapeHtml(message)}</span>
        <button class="toast-close" title="Cerrar">✕</button>
    `;
    toast.querySelector('.toast-close').addEventListener('click', () => toast.remove());
    container.appendChild(toast);
    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateX(20px)';
        setTimeout(() => toast.remove(), 250);
    }, 5000);
}

/** Cambia el título de la vista activa. */
function setPageTitle(title) {
    document.getElementById('page-title').textContent = title;
}

// ============================================================
// 4. COMPONENTES REUTILIZABLES
// ============================================================

/** Crea una tarjeta de estadística. */
function StatCard({ icon, value, label, colorClass = '' }) {
    const card = document.createElement('div');
    card.className = 'card';
    card.innerHTML = `
        <span class="card-icon ${colorClass}">${icon}</span>
        <span class="card-value">${escapeHtml(String(value))}</span>
        <span class="card-label">${escapeHtml(label)}</span>
    `;
    return card;
}

/** Renderiza una tabla genérica en el tbody de un selector dado. */
function renderTable(selector, rows) {
    const tbody = document.querySelector(`${selector} tbody`);
    tbody.innerHTML = '';
    if (!rows || rows.length === 0) {
        const cols = tbody.closest('table').querySelectorAll('thead th').length;
        tbody.innerHTML = `<tr><td colspan="${cols}" class="muted" style="text-align:center">No hay datos.</td></tr>`;
        return;
    }
    for (const row of rows) {
        const tr = document.createElement('tr');
        tr.innerHTML = row;
        tbody.appendChild(tr);
    }
}

/** Renderiza el extra_json de un archivo como lista clave-valor. */
function renderExtraJson(extraJson) {
    if (!extraJson) return '<p class="muted">No hay metadatos adicionales.</p>';
    try {
        const obj = JSON.parse(extraJson);
        if (obj && typeof obj === 'object' && !Array.isArray(obj)) {
            const dl = document.createElement('dl');
            dl.className = 'extra-list';
            for (const [key, value] of Object.entries(obj)) {
                const dt = document.createElement('dt');
                dt.textContent = key;
                const dd = document.createElement('dd');
                dd.textContent = formatExtraValue(value);
                dd.title = formatExtraValue(value);
                dl.appendChild(dt);
                dl.appendChild(dd);
            }
            return dl.outerHTML;
        }
    } catch (_) {
        // fall through
    }
    return `<p class="muted">Metadatos adicionales no estructurados.</p><pre class="raw-meta">${escapeHtml(extraJson)}</pre>`;
}

function formatExtraValue(value) {
    if (value == null) return '-';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
}

// ============================================================
// 5. MODAL DE ESCANEO
// ============================================================

function openScanModal() {
    document.getElementById('scan-modal').classList.remove('hidden');
    document.body.style.overflow = 'hidden';
    // Intenta sugerir una ruta amigable por defecto.
    const input = document.getElementById('scan-path');
    if (!input.value) {
        input.value = '';
        input.placeholder = 'C:/Users/Admin/Videos';
    }
    input.focus();
}

function closeScanModal() {
    document.getElementById('scan-modal').classList.add('hidden');
    document.body.style.overflow = '';
}

async function startScan() {
    const path = document.getElementById('scan-path').value.trim();
    const concurrency = Number(document.getElementById('scan-concurrency').value) || 4;
    const force = document.getElementById('scan-force').checked;

    if (!path) {
        showToast('Escribe la ruta de una carpeta para escanear', 'warning');
        return;
    }

    try {
        const data = await apiPost('/api/scan', { path, concurrency, force });
        closeScanModal();
        showToast(`Escaneo iniciado (job #${data.job_id})`, 'success');
        startScanPolling();
    } catch (err) {
        showToast('Error al iniciar escaneo: ' + err.message, 'error');
    }
}

/** Refresca dashboard y lista de archivos mientras haya un escaneo activo. */
function startScanPolling() {
    if (scanPollTimer) return;
    scanPollTimer = setInterval(async () => {
        try {
            const jobs = await apiGet('/api/scan-jobs');
            renderScanStatus(jobs);
            const running = (jobs || []).some(j => j.status === 'running');
            if (currentView === 'dashboard') {
                await renderDashboard();
            } else if (currentView === 'files') {
                await renderFiles();
            }
            if (!running) {
                clearInterval(scanPollTimer);
                scanPollTimer = null;
                showToast('Escaneo finalizado', 'success');
            }
        } catch (err) {
            console.error('Error consultando escaneos:', err);
        }
    }, 3000);
}

// ============================================================
// 6. NAVEGACIÓN PRINCIPAL
// ============================================================

/** Cambia a la vista solicitada y recarga sus datos. */
function switchView(view) {
    currentView = view;

    // Actualiza sidebar
    document.querySelectorAll('.nav-item').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.view === view);
    });

    // Actualiza vistas
    document.querySelectorAll('.view').forEach(el => {
        el.classList.toggle('active', el.id === `view-${view}`);
    });

    const titles = {
        dashboard: 'Dashboard',
        files: 'Archivos',
        groups: 'Grupos',
        reorganize: 'Reorganizar',
    };
    setPageTitle(titles[view] || view);

    if (view === 'dashboard') renderDashboard();
    if (view === 'files') renderFiles();
    if (view === 'groups') renderGroups();
    if (view === 'reorganize') renderReorganize();
}

// ============================================================
// 6. VISTA DASHBOARD
// ============================================================

async function renderDashboard() {
    let stats;
    try {
        stats = await apiGet('/api/stats');
        const onboarding = document.getElementById('dashboard-onboarding');
        const toolbar = document.getElementById('dashboard-toolbar');
        const cards = document.getElementById('dashboard-cards');
        const chartPanel = document.getElementById('type-chart-panel');

        if ((stats.total_files ?? 0) === 0) {
            onboarding.classList.remove('hidden');
            toolbar.classList.add('hidden');
            cards.classList.add('hidden');
            chartPanel.classList.add('hidden');
        } else {
            onboarding.classList.add('hidden');
            toolbar.classList.remove('hidden');
            cards.classList.remove('hidden');
            chartPanel.classList.remove('hidden');

            cards.innerHTML = '';
            cards.appendChild(StatCard({ icon: '📁', value: stats.total_files ?? 0, label: 'Archivos indexados' }));
            cards.appendChild(StatCard({ icon: '🎬', value: stats.video_files ?? 0, label: 'Videos' }));
            cards.appendChild(StatCard({ icon: '🎵', value: stats.audio_files ?? 0, label: 'Audio' }));
            cards.appendChild(StatCard({ icon: '📄', value: stats.pdf_files ?? 0, label: 'PDFs' }));
            cards.appendChild(StatCard({ icon: '🗜️', value: stats.archive_files ?? 0, label: 'Archivos comprimidos' }));
            cards.appendChild(StatCard({ icon: '💾', value: formatBytes(stats.total_size_bytes ?? 0), label: 'Tamaño total' }));
            cards.appendChild(StatCard({ icon: '🎭', value: stats.group_count ?? 0, label: 'Grupos' }));
            cards.appendChild(StatCard({ icon: '🕒', value: formatDate(stats.last_scan), label: 'Último escaneo' }));
        }
    } catch (err) {
        showToast('No se pudieron cargar las estadísticas: ' + err.message, 'error');
    }

    try {
        const payload = await apiGet('/api/stats/by-type');
        if ((stats?.total_files ?? 0) > 0) {
            renderTypeChart(payload.data || {});
        }
    } catch (err) {
        showToast('No se pudo cargar el breakdown por tipo: ' + err.message, 'error');
    }

    try {
        const jobs = await apiGet('/api/scan-jobs');
        renderScanJobs(jobs);
        renderScanStatus(jobs);
    } catch (err) {
        showToast('No se pudieron cargar los escaneos: ' + err.message, 'error');
    }
}

/** Muestra un aviso si hay un escaneo en curso o el último job falló. */
function renderScanStatus(jobs) {
    const bar = document.getElementById('scan-status-bar');
    if (!jobs || jobs.length === 0) {
        bar.classList.add('hidden');
        return;
    }
    const latest = jobs[0];
    if (latest.status === 'running') {
        bar.classList.remove('hidden');
        bar.innerHTML = `
            <span class="scan-pulse" aria-hidden="true"></span>
            <span>Escaneando <strong>${escapeHtml(latest.root_path)}</strong> (job #${latest.id})...</span>
        `;
    } else if (latest.status === 'failed' || latest.status === 'completed_with_errors') {
        bar.classList.remove('hidden');
        bar.innerHTML = `
            <span class="badge status-failed">Error</span>
            <span>El último escaneo (job #${latest.id}) finalizó con errores.</span>
        `;
    } else {
        bar.classList.add('hidden');
    }
}

/** Dibuja un gráfico de barras CSS con los conteos por tipo. */
function renderTypeChart(data) {
    const chart = document.getElementById('type-chart');
    chart.innerHTML = '';
    const types = ['video', 'audio', 'pdf', 'archive', 'unknown'];
    const colors = {
        video: '#22d3ee',
        audio: '#a78bfa',
        pdf: '#fbbf24',
        archive: '#34d399',
        unknown: '#94a3b8',
    };
    const max = Math.max(...types.map(t => data[t] ?? 0), 1);

    for (const type of types) {
        const value = data[type] ?? 0;
        const pct = (value / max) * 100;
        const item = document.createElement('div');
        item.className = 'bar-item';
        item.innerHTML = `
            <div class="bar-value">${value}</div>
            <div class="bar-track">
                <div class="bar-fill" style="height: ${pct}%; background: ${colors[type]}"></div>
            </div>
            <div class="bar-label">${type}</div>
        `;
        chart.appendChild(item);
    }
}

function renderScanJobs(jobs) {
    const rows = (jobs || []).slice(0, 10).map(j => `
        <td><span class="badge status-${j.status}">${escapeHtml(j.status)}</span></td>
        <td>${formatDate(j.started_at)}</td>
        <td>${formatDate(j.finished_at)}</td>
        <td title="${escapeHtml(j.root_path)}">${escapeHtml(j.root_path)}</td>
        <td>${j.files_found ?? 0}</td>
        <td>${j.files_indexed ?? 0}</td>
        <td>${j.errors ?? 0}</td>
    `);
    renderTable('#scan-jobs-table', rows);
}

// ============================================================
// 7. VISTA ARCHIVOS
// ============================================================

async function renderFiles() {
    let totalCount = 0;
    try {
        const data = await apiGet('/api/files/count');
        totalCount = data.count || 0;
    } catch (err) {
        showToast('Error al contar archivos: ' + err.message, 'error');
    }

    const onboarding = document.getElementById('files-onboarding');
    const toolbar = document.querySelector('.files-toolbar');
    const chips = document.getElementById('file-type-chips');
    const filters = document.querySelector('.filters-grid');
    const resultsBar = document.querySelector('.results-bar');
    const tableWrap = document.querySelector('#files-table').closest('.table-wrap');
    const pagination = document.querySelector('.pagination');
    const filesEmpty = document.getElementById('files-empty');

    if (totalCount === 0) {
        onboarding.classList.remove('hidden');
        toolbar.classList.add('hidden');
        chips.classList.add('hidden');
        filters.classList.add('hidden');
        resultsBar.classList.add('hidden');
        tableWrap.classList.add('hidden');
        pagination.classList.add('hidden');
        filesEmpty.classList.add('hidden');
        return;
    }

    onboarding.classList.add('hidden');
    toolbar.classList.remove('hidden');
    chips.classList.remove('hidden');
    filters.classList.remove('hidden');
    resultsBar.classList.remove('hidden');
    tableWrap.classList.remove('hidden');
    pagination.classList.remove('hidden');

    await loadFilterOptions();
    await applyFileFilters();
}

/** Carga extensiones y grupos disponibles para los selects. */
async function loadFilterOptions() {
    try {
        const payload = await apiGet('/api/extensions');
        fillSelect('filter-ext', payload.data || [], 'Cualquier extensión');
    } catch (err) {
        showToast('No se pudieron cargar extensiones', 'error');
    }

    try {
        const payload = await apiGet('/api/groups');
        const groups = payload.data || [];
        groupsMap.clear();
        for (const g of groups) groupsMap.set(String(g.id), g.name);
        fillSelect('filter-group', groups.map(g => ({ value: g.id, label: g.name })), 'Cualquier grupo');
    } catch (err) {
        showToast('No se pudieron cargar grupos', 'error');
    }
}

/** Rellena un <select> conservando la selección actual. */
function fillSelect(id, options, defaultLabel) {
    const select = document.getElementById(id);
    const current = select.value;
    select.innerHTML = `<option value="">${escapeHtml(defaultLabel)}</option>`;
    for (const opt of options) {
        const value = typeof opt === 'string' ? opt : opt.value;
        const label = typeof opt === 'string' ? opt : opt.label;
        const option = document.createElement('option');
        option.value = value;
        option.textContent = label;
        select.appendChild(option);
    }
    select.value = current;
}

/** Lee los filtros de la UI y reinicia la paginación. */
function readFileFilters() {
    const subs = document.getElementById('filter-subs').value;
    fileFilters = {
        name: document.getElementById('filter-name').value || null,
        extension: document.getElementById('filter-ext').value || null,
        file_type: fileFilters.file_type, // controlado por chips
        min_size: document.getElementById('filter-min').value || null,
        max_size: document.getElementById('filter-max').value || null,
        has_subtitles: subs === '' ? null : subs === 'true',
        group_id: document.getElementById('filter-group').value || null,
        modified_after: document.getElementById('filter-after').value || null,
        modified_before: document.getElementById('filter-before').value || null,
        sort_by: document.getElementById('filter-sort-by').value || 'name',
        sort_order: document.getElementById('filter-sort-order').value || 'asc',
    };
    filePage.offset = 0;
}

/** Construye los parámetros de consulta para /api/files. */
function buildFileParams() {
    const params = new URLSearchParams();
    params.set('limit', filePage.size);
    params.set('offset', filePage.offset);
    for (const [key, value] of Object.entries(fileFilters)) {
        if (value != null && value !== '') params.set(key, value);
    }
    return params;
}

/** Carga conteo y listado de archivos. */
async function applyFileFilters() {
    const params = buildFileParams();

    // Conteo
    const countParams = new URLSearchParams(params);
    countParams.delete('limit');
    countParams.delete('offset');
    try {
        const data = await apiGet('/api/files/count?' + countParams.toString());
        filePage.count = data.count || 0;
    } catch (err) {
        showToast('Error al contar archivos: ' + err.message, 'error');
        filePage.count = 0;
    }

    // Listado
    try {
        const data = await apiGet('/api/files?' + params.toString());
        renderFileRows(data.data || []);
    } catch (err) {
        showToast('Error al cargar archivos: ' + err.message, 'error');
        renderFileRows([]);
    }

    updateFilePagination();
}

/** Renderiza filas de archivos incluyendo acciones. */
function renderFileRows(files) {
    const tbody = document.querySelector('#files-table tbody');
    const empty = document.getElementById('files-empty');
    const table = document.getElementById('files-table');

    tbody.innerHTML = '';

    if (files.length === 0) {
        table.classList.add('hidden');
        empty.classList.remove('hidden');
        return;
    }

    table.classList.remove('hidden');
    empty.classList.add('hidden');

    for (const f of files) {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td><a href="#" data-id="${f.id}" title="Ver detalle">${escapeHtml(f.name)}</a></td>
            <td>${typeBadge(f.file_type)}</td>
            <td>${f.extension || '-'}</td>
            <td>${formatBytes(f.size_bytes)}</td>
            <td>${formatDuration(f.duration_seconds)}</td>
            <td>${f.width && f.height ? f.width + 'x' + f.height : '-'}</td>
            <td>${f.has_subtitles ? '✅' : '—'}</td>
            <td>${escapeHtml(groupName(f.group_id))}</td>
            <td>
                <button class="btn-small btn-primary" data-id="${f.id}" title="Ver detalle del archivo">Ver detalle</button>
                <button class="btn-small btn-secondary" data-notes="${f.id}" title="Ver historial de notas">Historial</button>
            </td>
        `;
        tr.querySelector('a').addEventListener('click', e => {
            e.preventDefault();
            openFileModal(f.id);
        });
        tr.querySelector('[data-id]').addEventListener('click', () => openFileModal(f.id));
        tr.querySelector('[data-notes]').addEventListener('click', () => {
            openFileModal(f.id);
            switchModalTab('notes');
        });
        tbody.appendChild(tr);
    }
}

/** Actualiza texto y botones de paginación. */
function updateFilePagination() {
    const start = filePage.count === 0 ? 0 : filePage.offset + 1;
    const end = Math.min(filePage.offset + filePage.size, filePage.count);
    document.getElementById('results-count').textContent = `Total: ${filePage.count} archivo(s)`;
    document.getElementById('page-info').textContent = `Mostrando ${start}-${end} de ${filePage.count}`;
    document.getElementById('btn-prev').disabled = filePage.offset === 0;
    document.getElementById('btn-next').disabled = filePage.offset + filePage.size >= filePage.count;
}

/** Limpia todos los filtros visuales. */
function clearFileFilters() {
    document.getElementById('filter-name').value = '';
    document.getElementById('filter-ext').value = '';
    document.getElementById('filter-subs').value = '';
    document.getElementById('filter-group').value = '';
    document.getElementById('filter-min').value = '';
    document.getElementById('filter-max').value = '';
    document.getElementById('filter-after').value = '';
    document.getElementById('filter-before').value = '';
    document.getElementById('filter-sort-by').value = 'name';
    document.getElementById('filter-sort-order').value = 'asc';
    document.querySelectorAll('.chip').forEach(c => c.classList.remove('active'));
    document.querySelector('.chip[data-type=""]').classList.add('active');
    fileFilters.file_type = null;
    readFileFilters();
    applyFileFilters();
}

// ============================================================
// 8. VISTA GRUPOS
// ============================================================

async function renderGroups() {
    const kind = document.getElementById('filter-group-kind').value || null;
    try {
        const params = new URLSearchParams();
        if (kind) params.set('kind', kind);
        const payload = await apiGet('/api/groups?' + params.toString());
        const groups = payload.data || [];
        // Mantenemos el mapa de grupos actualizado para mostrar nombres en otras vistas.
        groupsMap.clear();
        for (const g of groups) groupsMap.set(String(g.id), g.name);
        renderGroupCards(groups);
        document.getElementById('groups-count').textContent = `${groups.length} grupo(s)`;
    } catch (err) {
        showToast('Error al cargar grupos: ' + err.message, 'error');
    }
}

function renderGroupCards(groups) {
    const grid = document.getElementById('groups-grid');
    const empty = document.getElementById('groups-empty');
    grid.innerHTML = '';

    if (groups.length === 0) {
        empty.classList.remove('hidden');
        return;
    }
    empty.classList.add('hidden');

    for (const g of groups) {
        const card = document.createElement('div');
        card.className = 'group-card';
        card.innerHTML = `
            <span class="group-icon">${groupIcon(g.kind)}</span>
            <h4>${escapeHtml(g.name)}</h4>
            <span class="badge kind-${g.kind}">${escapeHtml(g.kind || 'other')}</span>
            <p>${g.file_count ?? 0} archivo(s)</p>
        `;
        card.addEventListener('click', () => showGroupFiles(g.id, g.name));
        grid.appendChild(card);
    }
}

async function showGroupFiles(groupId, groupNameLabel) {
    try {
        const payload = await apiGet('/api/groups/' + groupId + '/files');
        const files = payload.data || [];
        document.getElementById('group-files-title').textContent = `Archivos del grupo: ${escapeHtml(groupNameLabel || '')}`;
        renderGroupFileRows(files);
        const panel = document.getElementById('group-files-panel');
        panel.classList.remove('hidden');
        panel.scrollIntoView({ behavior: 'smooth' });
    } catch (err) {
        showToast('Error al cargar archivos del grupo: ' + err.message, 'error');
    }
}

function renderGroupFileRows(files) {
    const rows = files.map(f => `
        <td><a href="#" data-id="${f.id}">${escapeHtml(f.name)}</a></td>
        <td>${typeBadge(f.file_type)}</td>
        <td>${f.extension || '-'}</td>
        <td>${formatBytes(f.size_bytes)}</td>
        <td>${formatDuration(f.duration_seconds)}</td>
        <td>${f.width && f.height ? f.width + 'x' + f.height : '-'}</td>
        <td>${f.has_subtitles ? '✅' : '—'}</td>
    `);
    renderTable('#group-files-table', rows);
    document.querySelectorAll('#group-files-table tbody a').forEach(a => {
        a.addEventListener('click', e => {
            e.preventDefault();
            openFileModal(Number(a.dataset.id));
        });
    });
}

// ============================================================
// 9. MODAL DE DETALLE DE ARCHIVO
// ============================================================

function openFileModal(id) {
    selectedFileId = id;
    loadFileDetail(id);
    document.getElementById('file-modal').classList.remove('hidden');
    document.body.style.overflow = 'hidden';
}

function closeFileModal() {
    document.getElementById('file-modal').classList.add('hidden');
    document.body.style.overflow = '';
    selectedFileId = null;
}

function switchModalTab(tab) {
    document.querySelectorAll('.modal-tab').forEach(t => {
        t.classList.toggle('active', t.dataset.tab === tab);
    });
    document.querySelectorAll('.modal-panel').forEach(p => {
        p.classList.toggle('active', p.id === `tab-${tab}`);
    });
}

async function loadFileDetail(id) {
    try {
        const payload = await apiGet('/api/files/' + id);
        const f = payload.data;

        document.getElementById('modal-file-name').textContent = f.name;
        document.getElementById('modal-file-badges').innerHTML = `
            ${typeBadge(f.file_type)}
            <span class="badge">${formatBytes(f.size_bytes)}</span>
        `;

        const general = document.getElementById('detail-general');
        general.innerHTML = `
            <div class="detail-item"><strong>Ruta</strong><span title="${escapeHtml(f.path)}">${escapeHtml(f.path)}</span></div>
            <div class="detail-item"><strong>Nombre</strong><span>${escapeHtml(f.name)}</span></div>
            <div class="detail-item"><strong>Extensión</strong><span>${f.extension || '-'}</span></div>
            <div class="detail-item"><strong>Tamaño</strong><span>${formatBytes(f.size_bytes)}</span></div>
            <div class="detail-item"><strong>Modificado</strong><span>${formatDate(f.modified_at)}</span></div>
            <div class="detail-item"><strong>Indexado</strong><span>${formatDate(f.scanned_at)}</span></div>
            <div class="detail-item"><strong>Grupo</strong><span>${escapeHtml(groupName(f.group_id))}</span></div>
            <div class="detail-item"><strong>Duración</strong><span>${formatDuration(f.duration_seconds)}</span></div>
            <div class="detail-item"><strong>Resolución</strong><span>${f.width && f.height ? f.width + 'x' + f.height : '-'}</span></div>
            <div class="detail-item"><strong>Códec vídeo</strong><span>${f.video_codec || '-'}</span></div>
            <div class="detail-item"><strong>Códec audio</strong><span>${f.audio_codec || '-'}</span></div>
            <div class="detail-item"><strong>Pistas audio</strong><span>${f.audio_tracks || '-'}</span></div>
            <div class="detail-item"><strong>Pistas subtítulos</strong><span>${f.subtitle_tracks || '-'}</span></div>
            <div class="detail-item"><strong>Subtítulos</strong><span>${f.has_subtitles ? 'Sí' : 'No'}</span></div>
        `;

        document.getElementById('detail-extra').innerHTML = renderExtraJson(f.extra_json);
        document.getElementById('note-input').value = f.notes || '';

        await Promise.all([
            loadFileTags(id),
            loadNotesHistory(id),
        ]);
    } catch (err) {
        showToast('Error al cargar detalle: ' + err.message, 'error');
    }
}

async function loadFileTags(fileId) {
    try {
        const tags = await apiGet('/api/files/' + fileId + '/tags');
        renderTags(tags);
    } catch (err) {
        showToast('Error al cargar tags: ' + err.message, 'error');
    }
}

function renderTags(tags) {
    const container = document.getElementById('detail-tags');
    container.innerHTML = '';
    if (!tags || tags.length === 0) {
        container.innerHTML = '<p class="muted">Sin etiquetas.</p>';
        return;
    }
    for (const t of tags) {
        const chip = document.createElement('span');
        chip.className = 'tag-chip';
        chip.innerHTML = `${escapeHtml(t.name)} <button data-id="${t.id}" title="Eliminar etiqueta">×</button>`;
        chip.querySelector('button').addEventListener('click', () => removeTag(t.id));
        container.appendChild(chip);
    }
}

async function addTag() {
    if (selectedFileId == null) return;
    const input = document.getElementById('tag-input');
    const name = input.value.trim();
    if (!name) return;
    try {
        await apiPost('/api/files/' + selectedFileId + '/tags', { name });
        input.value = '';
        await loadFileTags(selectedFileId);
        showToast('Etiqueta añadida', 'success');
    } catch (err) {
        showToast('Error al añadir etiqueta: ' + err.message, 'error');
    }
}

async function removeTag(tagId) {
    if (selectedFileId == null) return;
    try {
        await apiDelete('/api/files/' + selectedFileId + '/tags/' + tagId);
        await loadFileTags(selectedFileId);
        showToast('Etiqueta eliminada', 'success');
    } catch (err) {
        showToast('Error al eliminar etiqueta: ' + err.message, 'error');
    }
}

async function loadNotesHistory(fileId) {
    try {
        const payload = await apiGet('/api/files/' + fileId + '/notes');
        renderNotesHistory(payload.data || []);
    } catch (err) {
        showToast('Error al cargar historial de notas: ' + err.message, 'error');
    }
}

function renderNotesHistory(notes) {
    const container = document.getElementById('notes-history');
    container.innerHTML = '';
    if (notes.length === 0) {
        container.innerHTML = '<p class="muted">Sin historial de notas.</p>';
        return;
    }
    const ul = document.createElement('ul');
    ul.className = 'notes-history-list';
    for (const n of notes) {
        const li = document.createElement('li');
        li.innerHTML = `
            <div class="note-meta">
                <span>${formatDate(n.created_at)}</span>
                <button class="btn-small btn-danger" data-id="${n.id}">Eliminar</button>
            </div>
            <div class="note-text">${escapeHtml(n.content)}</div>
        `;
        li.querySelector('button').addEventListener('click', () => deleteNote(n.id));
        ul.appendChild(li);
    }
    container.appendChild(ul);
}

async function deleteNote(noteId) {
    if (!confirm('¿Eliminar esta nota del historial?')) return;
    try {
        await apiDelete('/api/notes/' + noteId);
        if (selectedFileId != null) await loadNotesHistory(selectedFileId);
        showToast('Nota eliminada', 'success');
    } catch (err) {
        showToast('Error al eliminar nota: ' + err.message, 'error');
    }
}

async function saveNote() {
    if (selectedFileId == null) return;
    const content = document.getElementById('note-input').value;
    try {
        await apiPost('/api/files/' + selectedFileId + '/notes', { content });
        await loadNotesHistory(selectedFileId);
        showToast('Nota guardada', 'success');
    } catch (err) {
        showToast('Error al guardar nota: ' + err.message, 'error');
    }
}

// ============================================================
// 10. VISTA REORGANIZAR (wizard)
// ============================================================

async function renderReorganize() {
    // Cargamos grupos primero para que la vista previa de plantillas pueda usar nombres reales.
    await loadGroupsMap();
    await loadReorgStrategies();
    await loadReorgFilterOptions();
    await loadSystemStorage();
}

/** Carga todos los grupos y actualiza el mapa id -> nombre. */
async function loadGroupsMap() {
    try {
        const payload = await apiGet('/api/groups');
        const groups = payload.data || [];
        groupsMap.clear();
        for (const g of groups) groupsMap.set(String(g.id), g.name);
    } catch (err) {
        console.error('Error cargando grupos:', err);
    }
}

async function loadReorgStrategies() {
    try {
        const data = await apiGet('/api/reorganize/strategies');
        reorgStrategies = data.strategies || [];
        renderStrategyCards();
        renderTokenHelp(data.tokens || []);
    } catch (err) {
        showToast('Error al cargar estrategias: ' + err.message, 'error');
    }
}

function renderStrategyCards() {
    const grid = document.getElementById('strategy-grid');
    grid.innerHTML = '';
    const descriptions = {
        'by-type': 'Crea carpetas por tipo de archivo: video, audio, pdf...',
        'by-group': 'Usa el grupo detectado (serie, película, colección).',
        'by-date': 'Organiza por año y mes de modificación.',
        'by-tag': 'Agrupa archivos según su etiqueta asignada.',
        'custom': 'Tú eliges la plantilla y la estructura final.',
    };
    const examples = {
        'by-type': '{file_type}/{name}.{ext}',
        'by-group': '{group_kind}/{group_name}/{name}.{ext}',
        'by-date': '{year}/{month}/{name}.{ext}',
        'by-tag': '{tag}/{name}.{ext}',
        'custom': '{file_type}/{name}.{ext}',
    };
    for (const s of reorgStrategies) {
        const card = document.createElement('div');
        card.className = 'strategy-card';
        card.dataset.strategy = s.id;
        card.innerHTML = `
            <h4>${escapeHtml(s.name)}</h4>
            <p>${descriptions[s.id] || ''}</p>
            <code>${escapeHtml(examples[s.id] || s.template)}</code>
        `;
        card.addEventListener('click', () => selectReorgStrategy(s.id));
        grid.appendChild(card);
    }
    if (selectedReorgStrategy) highlightStrategy(selectedReorgStrategy);
}

function renderTokenHelp(tokens) {
    const help = document.querySelector('.token-help');
    if (!help || !tokens.length) return;
    help.innerHTML = '<strong>Tokens disponibles:</strong> ' +
        tokens.map(t => `<code>${escapeHtml(t)}</code>`).join('');
}

function selectReorgStrategy(id) {
    selectedReorgStrategy = id;
    highlightStrategy(id);
    const entry = reorgStrategies.find(s => s.id === id);
    if (entry && entry.template) {
        document.getElementById('reorg-template').value = entry.template;
    }
    updatePathPreview();
}

function highlightStrategy(id) {
    document.querySelectorAll('.strategy-card').forEach(c => {
        c.classList.toggle('selected', c.dataset.strategy === id);
    });
}

async function loadReorgFilterOptions() {
    try {
        const types = await apiGet('/api/file-types');
        fillSelect('reorg-filter-type', types.data || [], 'Cualquiera');
    } catch (err) {
        showToast('Error al cargar tipos', 'error');
    }
    try {
        const exts = await apiGet('/api/extensions');
        fillSelect('reorg-filter-ext', exts.data || [], 'Cualquiera');
    } catch (err) {
        showToast('Error al cargar extensiones', 'error');
    }
    try {
        const tags = await apiGet('/api/tags');
        fillSelect('reorg-filter-tag', (tags || []).map(t => ({ value: t.id, label: t.name })), 'Cualquiera');
    } catch (err) {
        showToast('Error al cargar etiquetas', 'error');
    }
}

async function loadSystemStorage() {
    try {
        const payload = await apiGet('/api/system/storage');
        const disks = payload.data || [];
        const rows = disks.map(d => `
            <td>${escapeHtml(d.name)}</td>
            <td>${escapeHtml(d.mount_point)}</td>
            <td>${formatBytes(d.total_bytes)}</td>
            <td>${formatBytes(d.free_bytes)}</td>
            <td>${formatBytes(d.used_bytes)}</td>
        `);
        renderTable('#system-storage-table', rows);
    } catch (err) {
        showToast('Error al cargar discos: ' + err.message, 'error');
    }
}

/**
 * Genera una vista previa de ruta usando un archivo real de ejemplo.
 * Solo es ilustrativa; el backend decide la ruta final al planificar.
 */
async function updatePathPreview() {
    const preview = document.getElementById('reorg-path-preview');
    const template = document.getElementById('reorg-template').value;
    const targetRoot = document.getElementById('reorg-target-root').value;

    if (!selectedReorgStrategy || !template) {
        preview.textContent = 'Selecciona una estrategia y rellena los campos para ver un ejemplo.';
        return;
    }

    // Pedimos un archivo de ejemplo aplicando los filtros actuales
    const params = new URLSearchParams();
    params.set('limit', '1');
    const fileType = document.getElementById('reorg-filter-type').value;
    const extension = document.getElementById('reorg-filter-ext').value;
    if (fileType) params.set('file_type', fileType);
    if (extension) params.set('extension', extension);

    try {
        const data = await apiGet('/api/files?' + params.toString());
        const files = data.data || [];
        if (files.length === 0) {
            preview.textContent = `${targetRoot || 'D:/Organizado'}/${template}`;
            return;
        }
        const f = files[0];
        const group = groupsMap.get(String(f.group_id));
        const date = new Date(f.modified_at || Date.now());
        const tokens = {
            '{file_type}': f.file_type || 'unknown',
            '{extension}': f.extension || 'bin',
            '{name}': f.name.replace(/\.[^.]+$/, '') || 'archivo',
            '{ext}': f.extension || 'bin',
            '{group_name}': group || 'sin-grupo',
            '{group_kind}': 'collection',
            '{year}': String(date.getFullYear()),
            '{month}': String(date.getMonth() + 1).padStart(2, '0'),
            '{day}': String(date.getDate()).padStart(2, '0'),
            '{tag}': 'etiqueta',
        };
        let path = template;
        for (const [tok, val] of Object.entries(tokens)) {
            path = path.replaceAll(tok, val);
        }
        preview.textContent = `${targetRoot ? targetRoot.replace(/\\/g, '/') + '/' : ''}${path}`;
    } catch (err) {
        preview.textContent = 'No se pudo cargar un archivo de ejemplo.';
    }
}

/** Avanza a un paso concreto del wizard. */
function goToStep(step) {
    document.querySelectorAll('.wizard-step').forEach(el => {
        const s = Number(el.dataset.step);
        el.classList.remove('active', 'completed');
        if (s === step) el.classList.add('active');
        else if (s < step) el.classList.add('completed');
    });
    document.querySelectorAll('.wizard-panel').forEach(el => {
        el.classList.toggle('active', Number(el.dataset.step) === step);
    });
}

async function createReorgPlan() {
    if (!selectedReorgStrategy) {
        showToast('Selecciona una estrategia primero', 'warning');
        return;
    }

    currentReorgJobId = null;
    document.getElementById('btn-reorg-apply').disabled = true;
    document.getElementById('btn-reorg-rollback').disabled = true;

    const request = {
        strategy: selectedReorgStrategy,
        template: document.getElementById('reorg-template').value,
        target_root: document.getElementById('reorg-target-root').value,
        allow_cross_volume: document.getElementById('reorg-cross-volume').checked,
        filter: {
            file_type: document.getElementById('reorg-filter-type').value || null,
            extension: document.getElementById('reorg-filter-ext').value || null,
            tag_id: document.getElementById('reorg-filter-tag').value ? parseInt(document.getElementById('reorg-filter-tag').value) : null,
        },
    };

    try {
        const data = await apiPost('/api/reorganize/plan', request);
        currentReorgJobId = data.job_id;
        renderSpaceEstimate(data.estimate);
        await loadReorgJobDetail(currentReorgJobId);
        goToStep(3);
        showToast(`Plan #${currentReorgJobId} generado`, 'success');
    } catch (err) {
        showToast('Error al generar plan: ' + err.message, 'error');
    }
}

function renderSpaceEstimate(estimate) {
    const container = document.getElementById('reorg-space-estimate');
    if (!estimate) {
        container.classList.add('hidden');
        return;
    }
    container.classList.remove('hidden');

    const total = estimate.total_source_bytes ?? 0;
    const extra = estimate.extra_bytes_required ?? 0;
    const free = estimate.target_free_bytes ?? 0;
    const targetTotal = estimate.target_total_bytes ?? 0;
    const used = targetTotal > free ? targetTotal - free : 0;

    document.getElementById('reorg-total-bytes').textContent = formatBytes(total);
    document.getElementById('reorg-extra-bytes').textContent = formatBytes(extra);
    document.getElementById('reorg-target-free').textContent = formatBytes(free);
    document.getElementById('reorg-target-total').textContent = formatBytes(targetTotal);

    // Barra visual
    const bar = document.getElementById('target-space-bar');
    bar.innerHTML = '';
    if (targetTotal > 0) {
        const usedPct = Math.min((used / targetTotal) * 100, 100);
        const requiredPct = Math.min((extra / targetTotal) * 100, 100 - usedPct);
        const freePct = Math.max(0, 100 - usedPct - requiredPct);
        bar.innerHTML = `
            <div class="space-used" style="width: ${usedPct}%"></div>
            <div class="space-required" style="width: ${requiredPct}%"></div>
            <div class="space-free" style="width: ${freePct}%"></div>
        `;
    }

    const adviceBox = document.getElementById('reorg-advice-box');
    adviceBox.textContent = estimate.advice || 'Sin consejos adicionales.';

    const warningsUl = document.getElementById('reorg-warnings');
    warningsUl.innerHTML = '';
    const warnings = estimate.warnings || [];
    if (warnings.length === 0) {
        warningsUl.innerHTML = '<li>✅ Sin advertencias.</li>';
    } else {
        for (const w of warnings) {
            const li = document.createElement('li');
            li.textContent = w;
            warningsUl.appendChild(li);
        }
    }

    const insufficient = free < extra;
    document.getElementById('btn-reorg-apply').disabled = insufficient;
}

async function loadReorgJobDetail(jobId) {
    try {
        const data = await apiGet('/api/reorganize/jobs/' + jobId);
        const ops = data.operations || [];
        const rows = ops.map(op => `
            <td><span class="badge status-${op.status}">${escapeHtml(op.status)}</span></td>
            <td>${escapeHtml(op.action)}</td>
            <td title="${escapeHtml(op.source_path)}">${escapeHtml(op.source_path)}</td>
            <td title="${escapeHtml(op.dest_path)}">${escapeHtml(op.dest_path)}</td>
            <td>${formatBytes(op.size_bytes)}</td>
            <td>${escapeHtml(op.error_message || '')}</td>
        `);
        const empty = document.getElementById('reorg-empty');
        const table = document.getElementById('reorg-operations-table');
        if (rows.length === 0) {
            table.classList.add('hidden');
            empty.classList.remove('hidden');
        } else {
            table.classList.remove('hidden');
            empty.classList.add('hidden');
            renderTable('#reorg-operations-table', rows);
        }
        return data;
    } catch (err) {
        showToast('Error al cargar detalle del job: ' + err.message, 'error');
    }
}

async function applyReorgPlan() {
    if (!currentReorgJobId) return;
    if (!confirm('⚠️ Se van a mover archivos físicamente. ¿Has hecho una copia de seguridad? ¿Continuar?')) return;
    try {
        const data = await apiPost('/api/reorganize/jobs/' + currentReorgJobId + '/apply', {});
        const detail = await loadReorgJobDetail(currentReorgJobId);
        goToStep(4);
        renderReorgResult(data.status || detail?.data?.status || 'unknown');
        showToast('Plan aplicado', 'success');
    } catch (err) {
        showToast('Error al aplicar plan: ' + err.message, 'error');
    }
}

async function rollbackReorgPlan() {
    if (!currentReorgJobId) return;
    if (!confirm('¿Revertir el último job de reorganización?')) return;
    try {
        const data = await apiPost('/api/reorganize/jobs/' + currentReorgJobId + '/rollback', {});
        await loadReorgJobDetail(currentReorgJobId);
        renderReorgResult(data.status || 'rolled_back');
        showToast('Rollback completado', 'success');
    } catch (err) {
        showToast('Error al revertir plan: ' + err.message, 'error');
    }
}

function renderReorgResult(status) {
    const box = document.getElementById('reorg-result');
    const ok = status === 'completed' || status === 'rolled_back';
    box.innerHTML = `
        <div class="empty-icon">${ok ? '✅' : '⚠️'}</div>
        <p><strong>Estado del job #${currentReorgJobId}:</strong> <span class="badge status-${status}">${escapeHtml(status)}</span></p>
        <p class="muted">${ok ? 'La operación finalizó correctamente.' : 'Revisa las operaciones con error en el paso anterior.'}</p>
    `;
    document.getElementById('btn-reorg-rollback').disabled = !['completed', 'failed'].includes(status);
}

function resetReorgWizard() {
    currentReorgJobId = null;
    selectedReorgStrategy = null;
    document.getElementById('reorg-target-root').value = '';
    document.getElementById('reorg-template').value = '{file_type}/{name}.{ext}';
    document.getElementById('reorg-cross-volume').checked = false;
    document.getElementById('reorg-filter-type').value = '';
    document.getElementById('reorg-filter-ext').value = '';
    document.getElementById('reorg-filter-tag').value = '';
    document.getElementById('reorg-space-estimate').classList.add('hidden');
    document.getElementById('reorg-operations-table').classList.add('hidden');
    document.getElementById('reorg-empty').classList.add('hidden');
    document.getElementById('btn-reorg-apply').disabled = true;
    document.getElementById('btn-reorg-rollback').disabled = true;
    renderStrategyCards();
    goToStep(1);
}

// ============================================================
// 11. EVENTOS E INICIALIZACIÓN
// ============================================================

document.addEventListener('DOMContentLoaded', () => {
    // Navegación del sidebar
    document.querySelectorAll('.nav-item').forEach(btn => {
        btn.addEventListener('click', () => switchView(btn.dataset.view));
    });

    // Colapsar sidebar
    document.getElementById('sidebar-toggle').addEventListener('click', () => {
        document.getElementById('sidebar').classList.toggle('collapsed');
    });

    // Ayuda rápida
    document.getElementById('btn-help').addEventListener('click', () => {
        showToast('Alexandria indexa archivos locales. Escanea, explora, etiqueta y reorganiza sin conexión.', 'info');
    });

    // --- Escanear carpeta ---
    document.getElementById('btn-first-scan').addEventListener('click', openScanModal);
    document.getElementById('btn-scan-another').addEventListener('click', openScanModal);
    document.getElementById('btn-scan-folder').addEventListener('click', openScanModal);
    document.getElementById('btn-files-first-scan').addEventListener('click', openScanModal);

    document.getElementById('btn-close-scan-modal').addEventListener('click', closeScanModal);
    document.getElementById('btn-cancel-scan').addEventListener('click', closeScanModal);
    document.getElementById('scan-modal').addEventListener('click', (e) => {
        if (e.target.id === 'scan-modal') closeScanModal();
    });
    document.getElementById('btn-start-scan').addEventListener('click', startScan);
    document.getElementById('scan-path').addEventListener('keydown', (e) => {
        if (e.key === 'Enter') startScan();
    });

    // --- Archivos ---
    document.getElementById('filter-name').addEventListener('input', () => {
        clearTimeout(searchDebounceTimer);
        searchDebounceTimer = setTimeout(() => {
            readFileFilters();
            applyFileFilters();
        }, 350);
    });

    document.querySelectorAll('.chip').forEach(chip => {
        chip.addEventListener('click', () => {
            document.querySelectorAll('.chip').forEach(c => c.classList.remove('active'));
            chip.classList.add('active');
            fileFilters.file_type = chip.dataset.type || null;
            filePage.offset = 0;
            applyFileFilters();
        });
    });

    ['filter-ext', 'filter-subs', 'filter-group', 'filter-sort-by', 'filter-sort-order'].forEach(id => {
        document.getElementById(id).addEventListener('change', () => {
            readFileFilters();
            applyFileFilters();
        });
    });

    ['filter-min', 'filter-max', 'filter-after', 'filter-before'].forEach(id => {
        document.getElementById(id).addEventListener('change', () => {
            readFileFilters();
            applyFileFilters();
        });
    });

    document.getElementById('btn-clear-filters').addEventListener('click', clearFileFilters);
    document.getElementById('btn-empty-clear').addEventListener('click', clearFileFilters);

    document.getElementById('btn-prev').addEventListener('click', () => {
        if (filePage.offset >= filePage.size) {
            filePage.offset -= filePage.size;
            applyFileFilters();
        }
    });

    document.getElementById('btn-next').addEventListener('click', () => {
        if (filePage.offset + filePage.size < filePage.count) {
            filePage.offset += filePage.size;
            applyFileFilters();
        }
    });

    document.getElementById('page-size').addEventListener('change', (e) => {
        filePage.size = Number(e.target.value);
        filePage.offset = 0;
        applyFileFilters();
    });

    // --- Grupos ---
    document.getElementById('filter-group-kind').addEventListener('change', renderGroups);
    document.getElementById('btn-close-group').addEventListener('click', () => {
        document.getElementById('group-files-panel').classList.add('hidden');
    });

    // --- Modal detalle ---
    document.getElementById('btn-close-modal').addEventListener('click', closeFileModal);
    document.getElementById('file-modal').addEventListener('click', (e) => {
        if (e.target.id === 'file-modal') closeFileModal();
    });
    document.querySelectorAll('.modal-tab').forEach(tab => {
        tab.addEventListener('click', () => switchModalTab(tab.dataset.tab));
    });
    document.getElementById('btn-save-note').addEventListener('click', saveNote);
    document.getElementById('btn-add-tag').addEventListener('click', addTag);
    document.getElementById('tag-input').addEventListener('keydown', (e) => {
        if (e.key === 'Enter') addTag();
    });

    // --- Reorganizar wizard ---
    document.querySelectorAll('[data-prev]').forEach(btn => {
        btn.addEventListener('click', () => goToStep(Number(btn.dataset.prev)));
    });
    document.getElementById('btn-reorg-plan').addEventListener('click', createReorgPlan);
    document.getElementById('btn-reorg-apply').addEventListener('click', applyReorgPlan);
    document.getElementById('btn-reorg-rollback').addEventListener('click', rollbackReorgPlan);
    document.getElementById('btn-new-plan').addEventListener('click', resetReorgWizard);

    ['reorg-template', 'reorg-target-root', 'reorg-filter-type', 'reorg-filter-ext'].forEach(id => {
        document.getElementById(id).addEventListener('input', updatePathPreview);
        document.getElementById(id).addEventListener('change', updatePathPreview);
    });

    // Carga inicial
    switchView('dashboard');

    // Si ya había un escaneo en curso al cargar la página, seguimos refrescando.
    (async () => {
        try {
            const jobs = await apiGet('/api/scan-jobs');
            if ((jobs || []).some(j => j.status === 'running')) {
                startScanPolling();
            }
        } catch (err) {
            console.error('No se pudo comprobar escaneos activos:', err);
        }
    })();
});
