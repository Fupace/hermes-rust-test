const API = '/api';

document.querySelectorAll('.nav-links a').forEach(link => {
  link.addEventListener('click', (e) => {
    e.preventDefault();
    const page = link.dataset.page;
    showPage(page);
    document.querySelectorAll('.nav-links a').forEach(l => l.classList.remove('active'));
    link.classList.add('active');
  });
});

function showPage(name) {
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.getElementById('page-' + name).classList.add('active');
  if (name === 'dashboard') loadDashboard();
  else if (name === 'pods') loadPods();
  else if (name === 'deployments') loadDeployments();
  else if (name === 'services') loadServices();
  else if (name === 'namespaces') loadNamespaces();
}

async function loadDashboard() {
  const statusDot = document.getElementById('clusterStatus');
  try {
    const resp = await fetch(API + '/cluster/summary');
    const data = await resp.json();
    if (data.success) {
      const d = data.data;
      document.getElementById('statNamespaces').textContent = d.namespaces;
      document.getElementById('statNodes').textContent = d.nodes;
      document.getElementById('statPods').textContent = d.pods;
      document.getElementById('statDeployments').textContent = d.deployments;
      document.getElementById('statServices').textContent = d.services;
      statusDot.className = 'status-dot online';
      statusDot.title = 'Connected';
    }
  } catch (e) {
    statusDot.className = 'status-dot offline';
    statusDot.title = 'Disconnected';
  }
}

async function loadPods() {
  const nsFilter = document.getElementById('podNsFilter').value || '';
  const resp = await fetch(API + '/pods?namespace=' + nsFilter);
  const data = await resp.json();
  const tbody = document.querySelector('#podsTable tbody');
  tbody.innerHTML = '';
  if (!data.success || !data.data) return;
  data.data.forEach(p => {
    const tr = document.createElement('tr');
    tr.innerHTML = [
      '<td class="res-name">', esc(p.name), '</td>',
      '<td>', esc(p.namespace), '</td>',
      '<td><span class="badge badge-', p.status.toLowerCase(), '">', esc(p.status), '</span></td>',
      '<td>', esc(p.node), '</td>',
      '<td>', p.restarts, '</td>',
      '<td>', esc(p.age), '</td>',
      '<td><button class="btn-sm" onclick="showLogs(\'', esc(p.namespace), '\',\'', esc(p.name), '\')">Logs</button></td>'
    ].join('');
    tbody.appendChild(tr);
  });
}

async function loadDeployments() {
  const nsFilter = document.getElementById('deployNsFilter').value || '';
  const resp = await fetch(API + '/deployments?namespace=' + nsFilter);
  const data = await resp.json();
  const tbody = document.querySelector('#deploymentsTable tbody');
  tbody.innerHTML = '';
  if (!data.success || !data.data) return;
  data.data.forEach(d => {
    const tr = document.createElement('tr');
    tr.innerHTML = '<td class="res-name">' + esc(d.name) + '</td><td>' + esc(d.namespace) + '</td><td>' + d.replicas + '</td><td>' + d.ready + '</td><td>' + d.available + '</td><td>' + esc(d.age) + '</td><td>' + esc(d.containers.join(', ')) + '</td>';
    tbody.appendChild(tr);
  });
}

async function loadServices() {
  const nsFilter = document.getElementById('svcNsFilter').value || '';
  const resp = await fetch(API + '/services?namespace=' + nsFilter);
  const data = await resp.json();
  const tbody = document.querySelector('#servicesTable tbody');
  tbody.innerHTML = '';
  if (!data.success || !data.data) return;
  data.data.forEach(s => {
    const tr = document.createElement('tr');
    tr.innerHTML = '<td class="res-name">' + esc(s.name) + '</td><td>' + esc(s.namespace) + '</td><td>' + esc(s.service_type) + '</td><td>' + esc(s.cluster_ip) + '</td><td>' + esc(s.ports.join(', ')) + '</td><td>' + esc(s.age) + '</td>';
    tbody.appendChild(tr);
  });
}

async function loadNamespaces() {
  const resp = await fetch(API + '/namespaces');
  const data = await resp.json();
  const tbody = document.querySelector('#namespacesTable tbody');
  tbody.innerHTML = '';
  if (!data.success || !data.data) return;

  const nsOpts = data.data.map(function(n) { return '<option value="' + esc(n.name) + '">' + esc(n.name) + '</option>'; }).join('');
  ['podNsFilter','deployNsFilter','svcNsFilter'].forEach(function(id) {
    document.getElementById(id).innerHTML = '<option value="">All Namespaces</option>' + nsOpts;
  });

  data.data.forEach(function(n) {
    const tr = document.createElement('tr');
    tr.innerHTML = '<td class="res-name">' + esc(n.name) + '</td><td><span class="badge badge-' + n.status.toLowerCase() + '">' + esc(n.status) + '</span></td><td>' + esc(n.age) + '</td>';
    tbody.appendChild(tr);
  });
}

async function showLogs(ns, pod) {
  document.getElementById('logsTitle').textContent = 'Logs: ' + ns + '/' + pod;
  document.getElementById('logsContent').textContent = 'Loading...';
  document.getElementById('logsModal').style.display = 'flex';
  const resp = await fetch(API + '/namespaces/' + ns + '/pods/' + pod + '/logs');
  const data = await resp.json();
  document.getElementById('logsContent').textContent = data.success ? data.data : 'Error: ' + (data.error || 'Unknown');
}

document.getElementById('closeLogs').addEventListener('click', function() {
  document.getElementById('logsModal').style.display = 'none';
});

document.getElementById('podNsFilter').addEventListener('change', function() { loadPods(); });
document.getElementById('deployNsFilter').addEventListener('change', function() { loadDeployments(); });
document.getElementById('svcNsFilter').addEventListener('change', function() { loadServices(); });

function esc(s) { return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

loadDashboard();
setInterval(loadDashboard, 30000);
