let currentCluster = null;

// ========== Init ==========
document.addEventListener('DOMContentLoaded', () => {
    loadClusters();
    document.getElementById('clusterSelect').addEventListener('change', onClusterChange);
    // Nav clicks
    document.querySelectorAll('.nav-links a').forEach(a => {
        a.addEventListener('click', e => { e.preventDefault(); showPage(a.dataset.page); });
    });
    // NS filter binds
    ['pod','deploy','svc','pvc','cm','sec'].forEach(k => {
        let el = document.getElementById(k+'Ns');
        if (el) el.addEventListener('change', () => showPage(currentPage));
    });
});

// ========== Navigation ==========
let currentPage = 'clusters';

function showPage(name) {
    if (name !== 'clusters' && !currentCluster) { name = 'clusters'; }
    currentPage = name;
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    let el = document.getElementById('page-' + name);
    if (el) el.classList.add('active');
    // Nav active
    document.querySelectorAll('.nav-links a').forEach(a => {
        a.classList.toggle('active', a.dataset.page === name);
    });
    // Show/hide nav based on cluster selection
    document.querySelectorAll('.nav-links a').forEach(a => {
        a.style.display = (name === 'clusters' || !currentCluster) ? 'none' : '';
    });
    if (currentCluster && name !== 'clusters') {
        document.querySelectorAll('.nav-links a').forEach(a => a.style.display = '');
    }
    // Load data
    if (name === 'dashboard') loadDashboard();
    else if (name === 'pods') loadPods();
    else if (name === 'deployments') loadDeployments();
    else if (name === 'services') loadServices();
    else if (name === 'nodes') loadNodes();
    else if (name === 'pvcs') loadPvcs();
    else if (name === 'configmaps') loadConfigMaps();
    else if (name === 'secrets') loadSecrets();
}

// ========== Cluster Management ==========
function onClusterChange() {
    let v = document.getElementById('clusterSelect').value;
    if (!v) { currentCluster = null; showPage('clusters'); return; }
    currentCluster = v;
    loadNamespaces();
    showPage('dashboard');
}

async function loadClusters() {
    let r = await fetch('/api/clusters').then(r => r.json());
    if (!r.success) return;
    let sel = document.getElementById('clusterSelect');
    // Keep first option, rebuild rest
    while (sel.options.length > 1) sel.remove(1);
    r.data.forEach(c => {
        let opt = document.createElement('option');
        opt.value = c.id; opt.textContent = c.name;
        sel.appendChild(opt);
    });
    renderClusterList(r.data);
}

function renderClusterList(clusters) {
    let html = clusters.length ? clusters.map(c =>
        `<div class="stat-card cluster-card" onclick="selectCluster('${c.id}')">
           <div class="stat-value" style="font-size:20px">${esc(c.name)}</div>
           <div class="stat-label">${esc(c.description||'')}<br><small>${(c.created_at||'').substring(0,10)}</small></div>
           <button class="btn-sm" style="margin-top:8px" onclick="event.stopPropagation();deleteCluster('${c.id}')">删除</button>
         </div>`
    ).join('') : '<p style="color:#8b949e;grid-column:1/-1">暂无集群，点击右上角"+ 添加"按钮添加</p>';
    document.getElementById('clusterList').innerHTML = '<div class="cluster-grid">'+html+'</div>';
}

function selectCluster(id) {
    document.getElementById('clusterSelect').value = id;
    onClusterChange();
}

function toggleAddForm() {
    let f = document.getElementById('addClusterForm');
    f.style.display = f.style.display === 'none' ? 'block' : 'none';
    showPage('clusters');
}

async function addCluster() {
    let name = document.getElementById('clusterName').value.trim();
    let kc = document.getElementById('clusterKubeconfig').value.trim();
    let desc = document.getElementById('clusterDesc').value.trim();
    if (!name || !kc) { alert('名称和 kubeconfig 不能为空'); return; }
    if (!kc.match(/^[A-Za-z0-9+/=]+$/)) { kc = btoa(kc); }
    let r = await fetch('/api/clusters', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body: JSON.stringify({name, kubeconfig: kc, description: desc})
    }).then(r => r.json());
    if (r.success) {
        // Add to dropdown without reload
        let sel = document.getElementById('clusterSelect');
        let opt = document.createElement('option');
        opt.value = r.id; opt.textContent = name;
        sel.appendChild(opt);
        sel.value = r.id;
        currentCluster = r.id;
        document.getElementById('addClusterForm').style.display = 'none';
        document.getElementById('clusterName').value = '';
        document.getElementById('clusterKubeconfig').value = '';
        document.getElementById('clusterDesc').value = '';
        loadClusters(); // refresh list including new one
        loadNamespaces();
        showPage('dashboard');
    } else {
        alert('添加失败: ' + (r.error || ''));
    }
}

async function deleteCluster(id) {
    if (!confirm('确定删除此集群?')) return;
    await fetch('/api/clusters/' + id, {method:'DELETE'});
    if (currentCluster === id) {
        currentCluster = null;
        document.getElementById('clusterSelect').value = '';
        showPage('clusters');
    }
    loadClusters();
}

// ========== Dashboard ==========
async function loadDashboard() {
    if (!currentCluster) return;
    let r = await fetch('/api/clusters/' + currentCluster + '/summary').then(r => r.json());
    if (!r.success) return;
    let d = r.data;
    let items = [
        ['命名空间', d.namespaces], ['节点', d.nodes], ['Pods', d.pods],
        ['Deployments', d.deployments], ['Services', d.services], ['PVCs', d.pvcs]
    ];
    document.getElementById('statsGrid').innerHTML = items.map(s =>
        `<div class="stat-card"><div class="stat-value">${s[1]}</div><div class="stat-label">${s[0]}</div></div>`
    ).join('');
    document.getElementById('clusterStatus').className = 'status-dot online';
}

async function loadNamespaces() {
    if (!currentCluster) return;
    let r = await fetch('/api/clusters/' + currentCluster + '/namespaces').then(r => r.json());
    if (!r.success || !r.data) return;
    let opts = r.data.map(n => `<option value="${esc(n.name)}">${esc(n.name)}</option>`).join('');
    ['pod','deploy','svc','pvc','cm','sec'].forEach(k => {
        let el = document.getElementById(k+'Ns');
        if (el) el.innerHTML = '<option value="">全部命名空间</option>' + opts;
    });
}

// ========== Resource loaders ==========
async function loadPods() {
    if (!currentCluster) return;
    let ns = document.getElementById('podNs').value || '';
    let r = await fetch('/api/clusters/' + currentCluster + '/pods?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#podsTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(p => {
        tbody.innerHTML += `<tr>
            <td class="res-name">${esc(p.name)}</td><td>${esc(p.namespace)}</td>
            <td><span class="badge badge-${p.status.toLowerCase()}">${esc(p.status)}</span></td>
            <td>${esc(p.node)}</td><td>${p.restarts}</td><td>${esc(p.age)}</td>
            <td>
                <button class="btn-sm" onclick="showLogs('${esc(p.namespace)}','${esc(p.name)}')">日志</button>
                <button class="btn-sm" onclick="showYaml('${esc(p.namespace)}','${esc(p.name)}','pod')">YAML</button>
            </td></tr>`;
    });
}

async function loadDeployments() {
    if (!currentCluster) return;
    let ns = document.getElementById('deployNs').value || '';
    let r = await fetch('/api/clusters/' + currentCluster + '/deployments?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#deploymentsTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(d => {
        tbody.innerHTML += `<tr>
            <td class="res-name">${esc(d.name)}</td><td>${esc(d.namespace)}</td>
            <td>${d.replicas}</td><td>${d.ready}</td><td>${d.available}</td>
            <td>${esc(d.age)}</td><td>${esc((d.containers||[]).join(', '))}</td></tr>`;
    });
}

async function loadServices() {
    if (!currentCluster) return;
    let ns = document.getElementById('svcNs').value || '';
    let r = await fetch('/api/clusters/' + currentCluster + '/services?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#servicesTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(s => {
        tbody.innerHTML += `<tr>
            <td class="res-name">${esc(s.name)}</td><td>${esc(s.namespace)}</td>
            <td>${esc(s.service_type)}</td><td>${esc(s.cluster_ip)}</td>
            <td>${esc((s.ports||[]).join(', '))}</td><td>${esc(s.age)}</td></tr>`;
    });
}

async function loadNodes() {
    if (!currentCluster) return;
    let r = await fetch('/api/clusters/' + currentCluster + '/nodes').then(r => r.json());
    let tbody = document.querySelector('#nodesTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(n => {
        tbody.innerHTML += `<tr>
            <td class="res-name">${esc(n.name)}</td>
            <td><span class="badge badge-${n.status.toLowerCase()}">${esc(n.status)}</span></td>
            <td>${esc(n.roles)}</td><td>${esc(n.version)}</td>
            <td>${esc(n.cpu)}</td><td>${esc(n.memory)}</td><td>${esc(n.age)}</td></tr>`;
    });
}

function makeSimpleLoader(tableId, apiPath, nsFilterId, fmt) {
    return async function() {
        if (!currentCluster) return;
        let ns = document.getElementById(nsFilterId)?.value || '';
        let r = await fetch('/api/clusters/' + currentCluster + '/' + apiPath + '?namespace=' + ns).then(r => r.json());
        let tbody = document.querySelector('#' + tableId + ' tbody');
        tbody.innerHTML = '';
        if (!r.success || !r.data) return;
        r.data.forEach(item => { tbody.innerHTML += fmt(item); });
    };
}

let loadPvcs = makeSimpleLoader('pvcsTable', 'pvcs', 'pvcNs', i =>
    `<tr><td class="res-name">${esc(i.name)}</td><td>${esc(i.namespace)}</td><td>${esc(i.status)}</td><td>${esc(i.capacity)}</td><td>${esc(i.storage_class)}</td><td>${esc(i.age)}</td></tr>`);

let loadConfigMaps = makeSimpleLoader('configmapsTable', 'configmaps', 'cmNs', i =>
    `<tr><td class="res-name">${esc(i.name)}</td><td>${esc(i.namespace)}</td><td>${esc((i.keys||[]).join(', '))}</td><td>${esc(i.age)}</td></tr>`);

let loadSecrets = makeSimpleLoader('secretsTable', 'secrets', 'secNs', i =>
    `<tr><td class="res-name">${esc(i.name)}</td><td>${esc(i.namespace)}</td><td>${esc(i.secret_type)}</td><td>${esc((i.keys||[]).join(', '))}</td><td>${esc(i.age)}</td></tr>`);

// ========== Modals ==========
async function showLogs(ns, pod) {
    document.getElementById('modalTitle').textContent = '日志: ' + ns + '/' + pod;
    document.getElementById('modalBody').textContent = 'Loading...';
    document.getElementById('modal').style.display = 'flex';
    let r = await fetch('/api/clusters/' + currentCluster + '/namespaces/' + ns + '/pods/' + pod + '/logs').then(r => r.json());
    document.getElementById('modalBody').textContent = r.success ? r.data : 'Error: ' + (r.error||'');
}

async function showYaml(ns, name, resource) {
    document.getElementById('modalTitle').textContent = resource + ': ' + ns + '/' + name;
    document.getElementById('modalBody').textContent = 'Loading...';
    document.getElementById('modal').style.display = 'flex';
    let r = await fetch('/api/clusters/' + currentCluster + '/namespaces/' + ns + '/' + resource + 's/' + name + '/yaml').then(r => r.json());
    document.getElementById('modalBody').textContent = r.success ? r.data : JSON.stringify(r, null, 2);
}

function closeModal() { document.getElementById('modal').style.display = 'none'; }

// ========== Create Deployment ==========
function showCreateDeploy() { document.getElementById('createDeployModal').style.display = 'flex'; }
function closeCreateDeploy() { document.getElementById('createDeployModal').style.display = 'none'; }

async function createDeployment() {
    let body = {
        name: document.getElementById('cdName').value.trim(),
        namespace: document.getElementById('cdNamespace').value.trim(),
        image: document.getElementById('cdImage').value.trim(),
        replicas: parseInt(document.getElementById('cdReplicas').value) || 1,
        port: parseInt(document.getElementById('cdPort').value) || 8080,
    };
    if (!body.name || !body.namespace || !body.image) { alert('请填写所有必填项'); return; }
    let r = await fetch('/api/clusters/' + currentCluster + '/deployments', {
        method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify(body)
    }).then(r => r.json());
    if (r.success) { closeCreateDeploy(); loadDeployments(); }
    else alert('创建失败: ' + (r.error||''));
}

// ========== Utilities ==========
function esc(s) { return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }
