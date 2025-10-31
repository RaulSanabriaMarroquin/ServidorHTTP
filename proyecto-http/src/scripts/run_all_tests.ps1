param(
  [string]$BaseUrl = "http://localhost:8080",
  [int]$NClients = 60,
  [int]$JobsPerPool = 8,
  [switch]$LightCPU,
  [switch]$SkipIO50MB
)

function Get-Json([string]$url) {
  try { (curl.exe -s $url) | ConvertFrom-Json } catch { $null }
}

Write-Host "== Smoke ==" 
$smoke = curl.exe -s "$BaseUrl/status"
if (-not $smoke) { throw "Server no responde en $BaseUrl" }

Write-Host "== Carrera (N=$NClients) ==" 
$jobs = @()
for ($i=0; $i -lt $NClients; $i++) {
  $jobs += Start-Job -ScriptBlock {
    param($u)
    & curl.exe -s -o $null -w "%{http_code}" "$u/reverse?text=x"
  } -ArgumentList $BaseUrl
}
Wait-Job -Job $jobs | Out-Null
$codes = $jobs | ForEach-Object { Receive-Job $_ } 
$jobs | Remove-Job -Force | Out-Null
$codes | Group-Object | Select-Object Name,Count | Format-Table | Out-Host

Write-Host "== Colas & Backpressure =="
$submitted = @()
for ($i=0; $i -lt $JobsPerPool; $i++) {
  $resp = curl.exe -s "$BaseUrl/jobs/submit?task=isprime&n=15485863&prio=normal"
  if ($resp) { $submitted += ($resp | ConvertFrom-Json) }
}
Start-Sleep -Milliseconds 300
$statuses = foreach($s in $submitted) { if ($s) { Get-Json "$BaseUrl/jobs/status?id=$($s.job_id)" } }
$statuses | Format-Table | Out-Host

Write-Host "== IO grande (50MB) =="
if (-not $SkipIO50MB) {
  $dataDir = "data"; if (-not (Test-Path $dataDir)) { New-Item -Type Directory $dataDir | Out-Null }
  $file = "big.txt"
  # 60MB determinísticos
  fsutil file createnew "$dataDir\$file" 62914560 | Out-Null
  $h = Get-Json "$BaseUrl/hashfile?name=$file&algo=sha256"
  $c = Get-Json "$BaseUrl/compress?name=$file&codec=gzip"
  $w = Get-Json "$BaseUrl/wordcount?name=$file"
  $h; $c; $w | Format-List | Out-Host
} else {
  Write-Host "(Saltado con -SkipIO50MB)"
}

Write-Host "== CPU varios segundos =="
if ($LightCPU) {
  $p = Get-Json "$BaseUrl/pi?digits=300"
  $m = Get-Json "$BaseUrl/matrixmul?size=120&seed=123"
} else {
  $p = Get-Json "$BaseUrl/pi?digits=800"
  $m = Get-Json "$BaseUrl/matrixmul?size=220&seed=123"
}
$p | Out-Host
$m | Out-Host

Write-Host "== Métricas & Status =="
$metrics = Get-Json "$BaseUrl/metrics"
$status  = Get-Json "$BaseUrl/status"
if ($status -and $metrics) {
  "accepted={0} handled={1}" -f $status.metrics.accepted,$status.metrics.handled | Out-Host
  $metrics.latency_ms | Out-Host
  $metrics.workers    | Out-Host
  $status.queues      | Out-Host

  Write-Host "== Validaciones (PASS/FAIL) =="
  $pass1 = ($status.metrics.accepted -eq $status.metrics.handled)
  $pendCount = 0
  foreach($q in $status.queues){ if ($q.pending -gt 0) { $pendCount++ } }
  $pass2 = ($pendCount -eq 0)
  if (-not $pass1) { Write-Host "FAIL: accepted != handled" -ForegroundColor Red }
  if (-not $pass2) { Write-Host "FAIL: hay pendientes en colas" -ForegroundColor Red }
  if ($pass1 -and $pass2) { Write-Host "OK: PASS" -ForegroundColor Green } else { exit 1 }
} else {
  Write-Host "No se pudieron leer /metrics o /status" -ForegroundColor Red
  exit 1
}
