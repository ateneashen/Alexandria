const pageSize = 25;
let currentOffset = 0;
let currentFilters = {};
let fileCount = 0;
let selectedFileId = null;
let groupsMap = new Map();
let currentTab = 'dashboard';

function formatBytes(bytes) {
    if (bytes === 0 || bytes == null) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatDuration(seconds) {
    if (seconds == null) return '-';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${h}h ${m}m ${s}s`;
}

function formatDate(iso) {
    if (!iso) return 'Nunca';
    return new Date(iso).toLocaleString();
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function typeBadge(fileType) {
    const type = (fileType || 'unknown').toLowerCase();
    return `<span class="badge type-${type}">${escapeHtml(type)}</span>`;
}

function groupName(id) {
    if (id == null) return '-';
    return groupsMap.get(id) || `Grupo ${id}`;
}

function switchTab(tab) {
    currentTab = tab;
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.tab === tab);
    });
    document.querySelectorAll('.tab-panel').forEach(panel => {
        panel.classList.toggle('hidden', panel.id !== `tab-${tab}`);
    });

    if (tab === 'dashboard') loadDashboard();
    if (tab === 'files') loadFiles();
    if (tab === 'groups') loadGroups();
    if (tab === 'reorganize') loadReorganizeTab();
}

async function loadDashboard() {
    try {
        const res = await fetch('/api/stats');
        const stats = await res.json();
        document.getElementById('dash-total').textContent = stats.total_files ?? 0;
        document.getElementById('dash-size').textContent = formatBytes(stats.total_size_bytes ?? 0);
        document.getElementById('dash-groups').textContent = stats.group_count ?? 0;
        document.getElementById('dash-scan').textContent = formatDate(stats.last_scan);
    } catch (err) {
        console.error('Error loading stats:', err);
    }

    try {
        const res = await fetch('/api/stats/by-type');
        const payload = await res.json();
        const data = payload.data || {};
        document.getElementById('type-video').textContent = data.video ?? 0;
        document.getElementById('type-audio').textContent = data.audio ?? 0;
        document.getElementById('type-pdf').textContent = data.pdf ?? 0;
        document.getElementById('type-archive').textContent = data.archive ?? 0;
        document.getElementById('type-unknown').textContent = data.unknown ?? 0;
    } catch (err) {
        console.error('Error loading by-type stats:', err);
    }

    try {
        const res = await fetch('/api/scan-jobs');
        const jobs = await res.json();
        renderScanJobs(jobs);
    } catch (err) {
        console.error('Error loading scan jobs:', err);
    }
}

function renderScanJobs(jobs) {
    const tbody = document.querySelector('#scan-jobs-table tbody');
    tbody.innerHTML = '';
    if (!jobs.length) {
        tbody.innerHTML = '<tr><td colspan="7" class="muted">No hay escaneos registrados.</td></tr>';
        return;
    }
    for (const j of jobs) {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td>${formatDate(j.started_at)}</td>
            <td>${formatDate(j.finished_at)}</td>
            <td title="${escapeHtml(j.root_path)}">${escapeHtml(j.root_path)}</td>
            <td>${j.files_found ?? 0}</td>
            <td>${j.files_indexed ?? 0}</td>
            <td>${j.errors ?? 0}</td>
            <td><span class="badge status-${j.status}">${escapeHtml(j.status)}</span></td>
        `;
        tbody.appendChild(tr);
    }
}

function buildFileFilterParams() {
    const params = new URLSearchParams();
    params.set('limit', pageSize);
    params.set('offset', currentOffset);
    if (currentFilters.name) params.set('name', currentFilters.name);
    if (currentFilters.extension) params.set('extension', currentFilters.extension);
    if (currentFilters.file_type) params.set('file_type', currentFilters.file_type);
    if (currentFilters.min_size) params.set('min_size', currentFilters.min_size);
    if (currentFilters.max_size) params.set('max_size', currentFilters.max_size);
    if (currentFilters.has_subtitles != null) params.set('has_subtitles', currentFilters.has_subtitles);
    if (currentFilters.group_id) params.set('group_id', currentFilters.group_id);
    if (currentFilters.modified_after) params.set('modified_after', currentFilters.modified_after);
    if (currentFilters.modified_before) params.set('modified_before', currentFilters.modified_before);
    if (currentFilters.sort_by) params.set('sort_by', currentFilters.sort_by);
    if (currentFilters.sort_order) params.set('sort_order', currentFilters.sort_order);
    return params;
}

async function loadFileCount() {
    const params = buildFileFilterParams();
    params.delete('limit');
    params.delete('offset');
    try {
        const res = await fetch('/api/files/count?' + params.toString());
        const data = await res.json();
        fileCount = data.count || 0;
        document.getElementById('results-count').textContent = `Total: ${fileCount} archivo(s)`;
        updatePageInfo();
    } catch (err) {
        console.error('Error loading file count:', err);
    }
}

function updatePageInfo() {
    const start = fileCount === 0 ? 0 : currentOffset + 1;
    const end = Math.min(currentOffset + pageSize, fileCount);
    document.getElementById('page-info').textContent = `Mostrando ${start}-${end} de ${fileCount}`;
}

async function loadFiles() {
    await loadFileCount();
    const params = buildFileFilterParams();
    try {
        const res = await fetch('/api/files?' + params.toString());
        const data = await res.json();
        renderFiles(data.data || [], '#files-table');
        updatePageInfo();
    } catch (err) {
        console.error('Error loading files:', err);
    }
}

function renderFiles(files, tableSelector) {
    const tbody = document.querySelector(`${tableSelector} tbody`);
    tbody.innerHTML = '';
    const isGroupTable = tableSelector === '#group-files-table';
    for (const f of files) {
        const tr = document.createElement('tr');
        let html = `
            <td><a href="#" data-id="${f.id}">${escapeHtml(f.name)}</a></td>
            <td>${f.extension || '-'}</td>
            <td>${typeBadge(f.file_type)}</td>
            <td>${formatBytes(f.size_bytes)}</td>
            <td>${formatDuration(f.duration_seconds)}</td>
            <td>${f.width && f.height ? f.width + 'x' + f.height : '-'}</td>
            <td>${f.has_subtitles ? 'Sí' : 'No'}</td>
        `;
        if (!isGroupTable) {
            html += `<td>${escapeHtml(groupName(f.group_id))}</td>`;
        }
        tr.innerHTML = html;
        tr.querySelector('a').addEventListener('click', (e) => {
            e.preventDefault();
            showDetail(f.id);
        });
        tbody.appendChild(tr);
    }
}

function readFilters() {
    const subs = document.getElementById('filter-subs').value;
    currentFilters = {
        name: document.getElementById('filter-name').value || null,
        extension: document.getElementById('filter-ext').value || null,
        file_type: document.getElementById('filter-file-type').value || null,
        min_size: document.getElementById('filter-min').value || null,
        max_size: document.getElementById('filter-max').value || null,
        has_subtitles: subs === '' ? null : subs === 'true',
        group_id: document.getElementById('filter-group').value || null,
        modified_after: document.getElementById('filter-after').value || null,
        modified_before: document.getElementById('filter-before').value || null,
        sort_by: document.getElementById('filter-sort-by').value || null,
        sort_order: document.getElementById('filter-sort-order').value || null,
    };
    currentOffset = 0;
}

async function loadFilterOptions() {
    try {
        const res = await fetch('/api/extensions');
        const payload = await res.json();
        const select = document.getElementById('filter-ext');
        const current = select.value;
        select.innerHTML = '<option value="">Cualquier extensión</option>';
        for (const ext of payload.data || []) {
            const opt = document.createElement('option');
            opt.value = ext;
            opt.textContent = ext;
            select.appendChild(opt);
        }
        select.value = current;
    } catch (err) {
        console.error('Error loading extensions:', err);
    }

    try {
        const res = await fetch('/api/file-types');
        const payload = await res.json();
        const select = document.getElementById('filter-file-type');
        const current = select.value;
        select.innerHTML = '<option value="">Cualquier tipo</option>';
        for (const ft of payload.data || []) {
            const opt = document.createElement('option');
            opt.value = ft;
            opt.textContent = ft;
            select.appendChild(opt);
        }
        select.value = current;
    } catch (err) {
        console.error('Error loading file types:', err);
    }
}

async function loadGroups() {
    const kind = document.getElementById('filter-group-kind').value || null;
    const params = new URLSearchParams();
    if (kind) params.set('kind', kind);
    try {
        const res = await fetch('/api/groups?' + params.toString());
        const data = await res.json();
        const groups = data.data || [];
        renderGroups(groups);
        updateGroupDropdown(groups);
    } catch (err) {
        console.error('Error loading groups:', err);
    }
}

function updateGroupDropdown(groups) {
    groupsMap.clear();
    for (const g of groups) {
        groupsMap.set(g.id, g.name);
    }
    const select = document.getElementById('filter-group');
    const current = select.value;
    select.innerHTML = '<option value="">Cualquier grupo</option>';
    for (const g of groups) {
        const opt = document.createElement('option');
        opt.value = g.id;
        opt.textContent = g.name;
        select.appendChild(opt);
    }
    select.value = current;
}

function renderGroups(groups) {
    const grid = document.getElementById('groups-grid');
    grid.innerHTML = '';
    if (groups.length === 0) {
        grid.innerHTML = '<p class="empty">No hay grupos.</p>';
        return;
    }
    for (const g of groups) {
        const card = document.createElement('div');
        card.className = 'group-card';
        card.innerHTML = `
            <h4>${escapeHtml(g.name)}</h4>
            <span class="badge kind-${g.kind}">${escapeHtml(g.kind || 'otro')}</span>
            <p>${g.file_count ?? 0} archivo(s)</p>
        `;
        card.addEventListener('click', () => showGroupFiles(g.id));
        grid.appendChild(card);
    }
}

async function showGroupFiles(groupId) {
    try {
        const res = await fetch('/api/groups/' + groupId + '/files');
        const data = await res.json();
        renderFiles(data.data || [], '#group-files-table');
        document.getElementById('group-files').classList.remove('hidden');
        document.getElementById('group-files').scrollIntoView({ behavior: 'smooth' });
    } catch (err) {
        console.error('Error loading group files:', err);
    }
}

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
        // fall through to plain text fallback
    }
    return `<p class="muted">Metadatos adicionales no estructurados.</p><pre class="raw-meta">${escapeHtml(extraJson)}</pre>`;
}

function formatExtraValue(value) {
    if (value == null) return '-';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
}

async function showDetail(id) {
    selectedFileId = id;
    try {
        const res = await fetch('/api/files/' + id);
        if (!res.ok) throw new Error('File not found');
        const payload = await res.json();
        const f = payload.data;

        const main = document.getElementById('detail-main');
        main.innerHTML = `
            <h4>Información general</h4>
            <p><strong>Ruta:</strong> <span title="${escapeHtml(f.path)}">${escapeHtml(f.path)}</span></p>
            <p><strong>Nombre:</strong> ${escapeHtml(f.name)}</p>
            <p><strong>Extensión:</strong> ${f.extension || '-'}</p>
            <p><strong>Tipo:</strong> ${typeBadge(f.file_type)}</p>
            <p><strong>Tamaño:</strong> ${formatBytes(f.size_bytes)}</p>
            <p><strong>Modificado:</strong> ${formatDate(f.modified_at)}</p>
            <p><strong>Escaneado:</strong> ${formatDate(f.scanned_at)}</p>
            <p><strong>Grupo:</strong> ${escapeHtml(groupName(f.group_id))}</p>
            <h4>Técnicos</h4>
            <p><strong>Duración:</strong> ${formatDuration(f.duration_seconds)}</p>
            <p><strong>Resolución:</strong> ${f.width && f.height ? f.width + 'x' + f.height : '-'}</p>
            <p><strong>Códec vídeo:</strong> ${f.video_codec || '-'}</p>
            <p><strong>Códec audio:</strong> ${f.audio_codec || '-'}</p>
            <p><strong>Pistas de audio:</strong> ${f.audio_tracks || '-'}</p>
            <p><strong>Pistas de subtítulos:</strong> ${f.subtitle_tracks || '-'}</p>
            <p><strong>Subtítulos:</strong> ${f.has_subtitles ? 'Sí' : 'No'}</p>
        `;

        const extra = document.getElementById('detail-extra');
        extra.innerHTML = '<h4>Metadatos adicionales</h4>' + renderExtraJson(f.extra_json);

        document.getElementById('detail-notes').innerHTML = f.notes
            ? `<div class="note-content">${escapeHtml(f.notes)}</div>`
            : '<p class="muted">Sin notas.</p>';
        document.getElementById('note-input').value = f.notes || '';

        document.getElementById('detail').classList.remove('hidden');
        document.getElementById('detail').scrollIntoView({ behavior: 'smooth' });

        await loadFileTags(id);
        await loadNotesHistory(id);
    } catch (err) {
        console.error('Error loading detail:', err);
    }
}

async function loadFileTags(fileId) {
    try {
        const res = await fetch('/api/files/' + fileId + '/tags');
        const tags = await res.json();
        renderTags(tags);
    } catch (err) {
        console.error('Error loading tags:', err);
    }
}

function renderTags(tags) {
    const container = document.getElementById('detail-tags');
    container.innerHTML = '';
    if (!tags.length) {
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
        const res = await fetch('/api/files/' + selectedFileId + '/tags', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name })
        });
        if (!res.ok) throw new Error('Failed to add tag');
        input.value = '';
        await loadFileTags(selectedFileId);
    } catch (err) {
        console.error('Error adding tag:', err);
        alert('Error al añadir etiqueta');
    }
}

async function removeTag(tagId) {
    if (selectedFileId == null) return;
    try {
        const res = await fetch('/api/files/' + selectedFileId + '/tags/' + tagId, {
            method: 'DELETE'
        });
        if (!res.ok) throw new Error('Failed to remove tag');
        await loadFileTags(selectedFileId);
    } catch (err) {
        console.error('Error removing tag:', err);
        alert('Error al eliminar etiqueta');
    }
}

async function loadNotesHistory(fileId) {
    try {
        const res = await fetch('/api/files/' + fileId + '/notes');
        const notes = await res.json();
        renderNotesHistory(notes);
    } catch (err) {
        console.error('Error loading notes history:', err);
    }
}

function renderNotesHistory(notes) {
    const container = document.getElementById('notes-history');
    container.innerHTML = '';
    if (!notes.length) {
        container.innerHTML = '<p class="muted">Sin historial de notas.</p>';
        return;
    }
    const ul = document.createElement('ul');
    ul.className = 'notes-history-list';
    for (const n of notes) {
        const li = document.createElement('li');
        li.innerHTML = `
            <div class="note-meta">${formatDate(n.created_at)}
                <button data-id="${n.id}" class="btn-small btn-danger">Eliminar</button>
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
        const res = await fetch('/api/notes/' + noteId, { method: 'DELETE' });
        if (!res.ok) throw new Error('Failed to delete note');
        if (selectedFileId != null) {
            await loadNotesHistory(selectedFileId);
        }
    } catch (err) {
        console.error('Error deleting note:', err);
        alert('Error al eliminar nota');
    }
}

async function saveNote() {
    if (selectedFileId == null) return;
    const content = document.getElementById('note-input').value;
    try {
        const res = await fetch('/api/files/' + selectedFileId + '/notes', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ content })
        });
        if (!res.ok) throw new Error('Failed to save note');
        document.getElementById('detail-notes').innerHTML = content
            ? `<div class="note-content">${escapeHtml(content)}</div>`
            : '<p class="muted">Sin notas.</p>';
        await loadNotesHistory(selectedFileId);
    } catch (err) {
        console.error('Error saving note:', err);
        alert('Error al guardar la nota');
    }
}

document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => switchTab(btn.dataset.tab));
});

document.getElementById('btn-search').addEventListener('click', () => {
    readFilters();
    loadFiles();
});

document.getElementById('btn-prev').addEventListener('click', () => {
    if (currentOffset >= pageSize) {
        currentOffset -= pageSize;
        loadFiles();
    }
});

document.getElementById('btn-next').addEventListener('click', () => {
    if (currentOffset + pageSize < fileCount) {
        currentOffset += pageSize;
        loadFiles();
    }
});

document.getElementById('btn-filter-groups').addEventListener('click', loadGroups);
document.getElementById('btn-save-note').addEventListener('click', saveNote);
document.getElementById('btn-add-tag').addEventListener('click', addTag);
document.getElementById('tag-input').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') addTag();
});

// Reorganize tab
document.getElementById('reorg-strategy').addEventListener('change', updateReorgTemplate);
document.getElementById('btn-reorg-plan').addEventListener('click', createReorgPlan);
document.getElementById('btn-reorg-apply').addEventListener('click', applyReorgPlan);
document.getElementById('btn-reorg-rollback').addEventListener('click', rollbackReorgPlan);

loadDashboard();
loadGroups();
loadFilterOptions();
loadReorganizeTab();

let reorgStrategies = [];
let currentReorgJobId = null;

async function loadReorganizeTab() {
    await loadReorgStrategies();
    await loadReorgFilterOptions();
    await loadSystemStorage();
}

async function loadReorgStrategies() {
    try {
        const res = await fetch('/api/reorganize/strategies');
        const data = await res.json();
        reorgStrategies = data.strategies || [];
        updateReorgTemplate();
    } catch (err) {
        console.error('Error loading reorg strategies:', err);
    }
}

function updateReorgTemplate() {
    const strategy = document.getElementById('reorg-strategy').value;
    const entry = reorgStrategies.find(s => s.id === strategy);
    if (entry && entry.template) {
        document.getElementById('reorg-template').value = entry.template;
    }
}

async function loadReorgFilterOptions() {
    try {
        const res = await fetch('/api/file-types');
        const payload = await res.json();
        const select = document.getElementById('reorg-filter-type');
        const current = select.value;
        select.innerHTML = '<option value="">Cualquiera</option>';
        for (const ft of payload.data || []) {
            const opt = document.createElement('option');
            opt.value = ft;
            opt.textContent = ft;
            select.appendChild(opt);
        }
        select.value = current;
    } catch (err) {
        console.error('Error loading reorg file types:', err);
    }

    try {
        const res = await fetch('/api/extensions');
        const payload = await res.json();
        const select = document.getElementById('reorg-filter-ext');
        const current = select.value;
        select.innerHTML = '<option value="">Cualquiera</option>';
        for (const ext of payload.data || []) {
            const opt = document.createElement('option');
            opt.value = ext;
            opt.textContent = ext;
            select.appendChild(opt);
        }
        select.value = current;
    } catch (err) {
        console.error('Error loading reorg extensions:', err);
    }

    try {
        const res = await fetch('/api/tags');
        const tags = await res.json();
        const select = document.getElementById('reorg-filter-tag');
        const current = select.value;
        select.innerHTML = '<option value="">Cualquiera</option>';
        for (const t of tags || []) {
            const opt = document.createElement('option');
            opt.value = t.id;
            opt.textContent = t.name;
            select.appendChild(opt);
        }
        select.value = current;
    } catch (err) {
        console.error('Error loading reorg tags:', err);
    }
}

async function createReorgPlan() {
    currentReorgJobId = null;
    document.getElementById('btn-reorg-apply').disabled = true;
    document.getElementById('btn-reorg-rollback').disabled = true;
    document.getElementById('reorg-space-estimate').classList.add('hidden');

    const request = {
        strategy: document.getElementById('reorg-strategy').value,
        template: document.getElementById('reorg-template').value,
        target_root: document.getElementById('reorg-target-root').value,
        allow_cross_volume: document.getElementById('reorg-cross-volume').checked,
        filter: {
            file_type: document.getElementById('reorg-filter-type').value || null,
            extension: document.getElementById('reorg-filter-ext').value || null,
            tag_id: document.getElementById('reorg-filter-tag').value ? parseInt(document.getElementById('reorg-filter-tag').value) : null,
        }
    };

    try {
        const res = await fetch('/api/reorganize/plan', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(request)
        });
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || 'Plan failed');
        currentReorgJobId = data.job_id;
        renderSpaceEstimate(data.estimate);
        await loadReorgJobDetail(currentReorgJobId);
        document.getElementById('reorg-status').textContent = `Plan generado: job #${currentReorgJobId}`;
    } catch (err) {
        console.error('Error creating reorg plan:', err);
        alert('Error al generar el plan: ' + err.message);
    }
}

async function loadSystemStorage() {
    try {
        const res = await fetch('/api/system/storage');
        const payload = await res.json();
        renderSystemStorage(payload.data || []);
    } catch (err) {
        console.error('Error loading system storage:', err);
    }
}

function renderSystemStorage(disks) {
    const tbody = document.querySelector('#system-storage-table tbody');
    tbody.innerHTML = '';
    if (!disks.length) {
        tbody.innerHTML = '<tr><td colspan="5" class="muted">No se detectaron discos.</td></tr>';
        return;
    }
    for (const d of disks) {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td>${escapeHtml(d.name)}</td>
            <td>${escapeHtml(d.mount_point)}</td>
            <td>${formatBytes(d.total_bytes)}</td>
            <td>${formatBytes(d.free_bytes)}</td>
            <td>${formatBytes(d.used_bytes)}</td>
        `;
        tbody.appendChild(tr);
    }
}

function renderSpaceEstimate(estimate) {
    const container = document.getElementById('reorg-space-estimate');
    if (!estimate) {
        container.classList.add('hidden');
        return;
    }
    container.classList.remove('hidden');
    document.getElementById('reorg-total-bytes').textContent = formatBytes(estimate.total_source_bytes ?? 0);
    document.getElementById('reorg-extra-bytes').textContent = formatBytes(estimate.extra_bytes_required ?? 0);
    document.getElementById('reorg-target-total').textContent = formatBytes(estimate.target_total_bytes ?? 0);
    document.getElementById('reorg-target-free').textContent = formatBytes(estimate.target_free_bytes ?? 0);
    const used = (estimate.target_total_bytes ?? 0) - (estimate.target_free_bytes ?? 0);
    document.getElementById('reorg-target-used').textContent = formatBytes(used);
    document.getElementById('reorg-advice').textContent = estimate.advice || '-';
    const warningsUl = document.getElementById('reorg-warnings');
    warningsUl.innerHTML = '';
    const warnings = estimate.warnings || [];
    if (warnings.length === 0) {
        warningsUl.innerHTML = '<li class="muted">Sin advertencias.</li>';
    } else {
        for (const w of warnings) {
            const li = document.createElement('li');
            li.textContent = w;
            warningsUl.appendChild(li);
        }
    }
    const insufficient = (estimate.target_free_bytes ?? 0) < (estimate.extra_bytes_required ?? 0);
    document.getElementById('btn-reorg-apply').disabled = insufficient;
}

async function loadReorgJobDetail(jobId) {
    try {
        const res = await fetch(`/api/reorganize/jobs/${jobId}`);
        const data = await res.json();
        const operations = data.operations || [];
        renderReorgOperations(operations);
        if (data.data && (data.data.status === 'completed' || data.data.status === 'failed')) {
            document.getElementById('btn-reorg-rollback').disabled = false;
        }
    } catch (err) {
        console.error('Error loading reorg job detail:', err);
    }
}

function renderReorgOperations(operations) {
    const container = document.getElementById('reorg-preview');
    const tbody = document.querySelector('#reorg-operations-table tbody');
    tbody.innerHTML = '';
    if (!operations.length) {
        container.classList.add('hidden');
        return;
    }
    container.classList.remove('hidden');
    for (const op of operations) {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td><span class="badge status-${op.status}">${escapeHtml(op.status)}</span></td>
            <td>${escapeHtml(op.action)}</td>
            <td title="${escapeHtml(op.source_path)}">${escapeHtml(op.source_path)}</td>
            <td title="${escapeHtml(op.dest_path)}">${escapeHtml(op.dest_path)}</td>
            <td>${formatBytes(op.size_bytes)}</td>
            <td>${escapeHtml(op.error_message || '')}</td>
        `;
        tbody.appendChild(tr);
    }
}

async function applyReorgPlan() {
    if (!currentReorgJobId) return;
    if (!confirm('⚠️ Se van a mover archivos físicamente. ¿Has hecho una copia de seguridad? ¿Continuar?')) return;
    try {
        const res = await fetch(`/api/reorganize/jobs/${currentReorgJobId}/apply`, { method: 'POST' });
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || 'Apply failed');
        await loadReorgJobDetail(currentReorgJobId);
        document.getElementById('reorg-status').textContent = `Job #${currentReorgJobId} estado: ${data.status}`;
    } catch (err) {
        console.error('Error applying reorg plan:', err);
        alert('Error al aplicar el plan: ' + err.message);
    }
}

async function rollbackReorgPlan() {
    if (!currentReorgJobId) return;
    if (!confirm('¿Revertir el último job de reorganización?')) return;
    try {
        const res = await fetch(`/api/reorganize/jobs/${currentReorgJobId}/rollback`, { method: 'POST' });
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || 'Rollback failed');
        await loadReorgJobDetail(currentReorgJobId);
        document.getElementById('reorg-status').textContent = `Job #${currentReorgJobId} estado: ${data.status}`;
    } catch (err) {
        console.error('Error rolling back reorg plan:', err);
        alert('Error al revertir el plan: ' + err.message);
    }
}
