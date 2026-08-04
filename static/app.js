let currentCluster = null;
let currentPage = 'clusters';

// Init
loadClusters();
document.getElementById('clusterSelect').addEventListener('change', function(e){
    if(e.target.value === '') { showPage('clusters'); toggleAddForm(); return; }
    currentCluster = e.target.value;
    showPage('dashboard');
    loadDashboard();
    loadNamespaces();
});

function showPage(name) {
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    let el = document.getElementById('page-' + name);
    if (el) el.classList.add('active');
    currentPage = name;
    if (name === 'dashboard') loadDashboard();
    else if (name === 'pods') loadPods();
    else if (name === 'deployments') loadDeployments();
    else if (name === 'services') loadServices();
    else if (name === 'nodes') loadNodes();
    else if (name === 'pvcs') loadPvcs();
    else if (name === 'configmaps') loadConfigMaps();
    else if (name === 'secrets') loadSecrets();
}

async function loadClusters() {
    let r = await fetch('/api/clusters').then(r => r.json());
    if (!r.success) return;
    let sel = document.getElementById('clusterSelect');
    r.data.forEach(c => {
        let opt = document.createElement('option');
        opt.value = c.id;
        opt.textContent = c.name;
        sel.appendChild(opt);
    });
    renderClusterList(r.data);
}

function renderClusterList(clusters) {
    let html = clusters.length ? clusters.map(c =>
        `<div class="stat-card cluster-card" onclick="selectCluster('${c.id}')">
           <div class="stat-value" style="font-size:20px">${esc(c.name)}</div>
           <div class="stat-label">${esc(c.description||'')}<br><small>${c.created_at?c.created_at.substring(0,10):''}</small></div>
           <button class="btn-sm" style="margin-top:8px" onclick="event.stopPropagation();deleteCluster('${c.id}')">删除</button>
         </div>`
    ).join('') : '<p style="color:#8b949e">暂无集群，请添加</p>';
    document.getElementById('clusterList').innerHTML = html;
}

function selectCluster(id) {
    document.getElementById('clusterSelect').value = id;
    currentCluster = id;
    showPage('dashboard');
    loadDashboard();
    loadNamespaces();
}

function toggleAddForm() {
    let f = document.getElementById('addClusterForm');
    f.style.display = f.style.display === 'none' ? 'block' : 'none';
    document.getElementById('page-clusters').classList.add('active');
}

async function addCluster() {
    let name = document.getElementById('clusterName').value.trim();
    let kc = document.getElementById('clusterKubeconfig').value.trim();
    let desc = document.getElementById('clusterDesc').value.trim();
    if (!name || !kc) { alert('名称和 kubeconfig 不能为空'); return; }
    // Auto base64 encode if needed
    if (!kc.match(/^[A-Za-z0-9+/=]+$/)) { kc = btoa(kc); }
    let r = await fetch('/api/clusters', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body: JSON.stringify({name, kubeconfig: kc, description: desc})
    }).then(r => r.json());
    if (r.success) location.reload();
    else alert('失败: ' + r.error);
}

async function deleteCluster(id) {
    if (!confirm('确定删除此集群?')) return;
    await fetch('/api/clusters/' + id, {method:'DELETE'});
    location.reload();
}

// Dashboard
async function loadDashboard() {
    if (!currentCluster) return;
    let r = await fetch('/api/clusters/' + currentCluster + '/summary').then(r => r.json());
    if (!r.success) return;
    let d = r.data;
    let stats = [
        ['命名空间', d.namespaces], ['节点', d.nodes], ['Pods', d.pods],
        ['Deployments', d.deployments], ['Services', d.services], ['PVCs', d.pvcs]
    ];
    document.getElementById('statsGrid').innerHTML = stats.map(s =>
        `<div class="stat-card"><div class="stat-value">${s[1]}</div><div class="stat-label">${s[0]}</div></div>`
    ).join('');
    document.getElementById('clusterStatus').className = 'status-dot online';
}

async function loadNamespaces() {
    if (!currentCluster) return;
    let r = await fetch('/api/clusters/' + currentCluster + '/namespaces').then(r => r.json());
    if (!r.success) return;
    let opts = r.data.map(n => `<option value="${esc(n.name)}">${esc(n.name)}</option>`).join('');
    ['podNs','deployNs','svcNs','pvcNs','cmNs','secNs'].forEach(id => {
        let el = document.getElementById(id);
        if (el) el.innerHTML = '<option value="">全部命名空间</option>' + opts;
    });
}

async function loadPods() {
    if (!currentCluster) return;
    let ns = document.getElementById('podNs').value;
    let r = await fetch('/api/clusters/' + currentCluster + '/pods?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#podsTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(p => {
        let tr = document.createElement('tr');
        tr.innerHTML = `<td class="res-name">${esc(p.name)}</td><td>${esc(p.namespace)}</td><td><span class="badge badge-${p.status.toLowerCase()}">${esc(p.status)}</span></td><td>${esc(p.node)}</td><td>${p.restarts}</td><td>${esc(p.age)}</td><td><button class="btn-sm" onclick="showLogs('${esc(p.namespace)}','${esc(p.name)}')">日志</button> <button class="btn-sm" onclick="showYaml('${esc(p.namespace)}','${esc(p.name)}','pod')">YAML</button></td>`;
        tbody.appendChild(tr);
    });
}

async function loadDeployments() {
    if (!currentCluster) return;
    let ns = document.getElementById('deployNs').value;
    let r = await fetch('/api/clusters/' + currentCluster + '/deployments?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#deploymentsTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(d => {
        let tr = document.createElement('tr');
        tr.innerHTML = `<td class="res-name">${esc(d.name)}</td><td>${esc(d.namespace)}</td><td>${d.replicas}</td><td>${d.ready}</td><td>${d.available}</td><td>${esc(d.age)}</td><td>${esc(d.containers.join(', '))}</td>`;
        tbody.appendChild(tr);
    });
}

async function loadServices() {
    if (!currentCluster) return;
    let ns = document.getElementById('svcNs').value;
    let r = await fetch('/api/clusters/' + currentCluster + '/services?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#servicesTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(s => {
        let tr = document.createElement('tr');
        tr.innerHTML = `<td class="res-name">${esc(s.name)}</td><td>${esc(s.namespace)}</td><td>${esc(s.service_type)}</td><td>${esc(s.cluster_ip)}</td><td>${esc(s.ports.join(', '))}</td><td>${esc(s.age)}</td>`;
        tbody.appendChild(tr);
    });
}

async function loadNodes() {
    if (!currentCluster) return;
    let r = await fetch('/api/clusters/' + currentCluster + '/nodes').then(r => r.json());
    let tbody = document.querySelector('#nodesTable tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(n => {
        let tr = document.createElement('tr');
        tr.innerHTML = `<td class="res-name">${esc(n.name)}</td><td><span class="badge badge-${n.status.toLowerCase()}">${esc(n.status)}</span></td><td>${esc(n.roles)}</td><td>${esc(n.version)}</td><td>${esc(n.cpu)}</td><td>${esc(n.memory)}</td><td>${esc(n.age)}</td>`;
        tbody.appendChild(tr);
    });
}

let _loadSimple = (tableId, apiPath, nsFilterId, columns) => async function() {
    if (!currentCluster) return;
    let ns = document.getElementById(nsFilterId)?.value || '';
    let r = await fetch('/api/clusters/' + currentCluster + '/' + apiPath + '?namespace=' + ns).then(r => r.json());
    let tbody = document.querySelector('#' + tableId + ' tbody');
    tbody.innerHTML = '';
    if (!r.success || !r.data) return;
    r.data.forEach(item => {
        let tr = document.createElement('tr');
        tr.innerHTML = columns.map(c => typeof c === 'function' ? c(item) : `<td>${esc(String(item[c]||''))}</td>`).join('');
        tbody.appendChild(tr);
    });
};

let loadPvcs = _loadSimple('pvcsTable', 'pvcs', 'pvcNs', ['name','namespace','status','capacity','storage_class','age']);
let loadConfigMaps = _loadSimple('configmapsTable', 'configmaps', 'cmNs', ['name','namespace', item => `<td>${esc((item.keys||[]).join(', '))}</td>`,'age']);
let loadSecrets = _loadSimple('secretsTable', 'secrets', 'secNs', ['name','namespace','secret_type', item => `<td>${esc((item.keys||[]).join(', '))}</td>`,'age']);

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
    document.getElementById('modalBody').textContent = r.success ? r.data : 'JSON: ' + JSON.stringify(r,null,2);
}

function closeModal() { document.getElementById('modal').style.display = 'none'; }

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

function esc(s) { return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

// Setup nav
['dashboard','pods','deployments','services','nodes','pvcs','configmaps','secrets'].forEach(name => {
    let a = document.createElement('a');
    a.href = '#'; a.textContent = {'dashboard':'概览','pods':'Pods','deployments':'Deployments','services':'Services','nodes':'节点','pvcs':'PVC','configmaps':'ConfigMap','secrets':'Secrets'}[name];
    a.addEventListener('click', e => { e.preventDefault(); showPage(name); });
    document.getElementById('navLinks').appendChild(a);
});
