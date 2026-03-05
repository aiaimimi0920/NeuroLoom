$ErrorActionPreference = 'Continue'
$base = 'InfiniteRefill/客户端/普通用户'
$acc = "$base/accounts"
$script = "$base/unix/单次续杯.sh"
$url = 'https://refill.aiaimimi.com'

$keys = @(
'k_CkN6gXxzVuIlRWvxWQl4_BtsmACGM1Vo',
'k_7-tlV45Fili-ret4Fg8Q6hwaqiV5mJp8',
'k_uJflA3HRZVvh0V0hDm2T1-2Bdf11DraT',
'k_kGUokdVbphKnOha1CVmq-QtQe0kRdfsI',
'k_8iIKlXyiwbBUPZUHRd_Xg_RGRLntHCu5',
'k_jbf_GnPb0xLgh5nDx2PLd37XqJKkhSLD',
'k_uG0Q94SbiIYzF9RAHkApC-edqopEe0rC',
'k_-_u2A7yIPAlcXIVhzv_bOe-JRT7B_wqM',
'k_IZLaBGymv7sG3-U_UdKw_XuYx3KPWPlr',
'k_fExAXW4kkQAWEtFtmtYTIGHQwB1MU3St',
'k_ngQ1rH6kBw9YJDBGSLyX56tJDQs78bkm',
'k_SY_UaLcrL7FRk-337p88sm4SgztiOoG6',
'k_ULZZnDqOLZL0Z4CGXwsHc0_xLXz0zLiF'
)

$out = @()
$idx = 0
foreach ($k in $keys) {
  $idx++
  Get-ChildItem -LiteralPath $acc -File -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
  $log = "tmp_key_test_case_$idx.log"
  & bash $script --sync-all $url $k *> $log
  $rc = $LASTEXITCODE
  $count = (Get-ChildItem -LiteralPath $acc -Filter *.json -File -ErrorAction SilentlyContinue).Count
  $status = if ($rc -eq 0) { 'PASS' } else { 'FAIL' }
  $out += [PSCustomObject]@{
    case = $idx
    key_prefix = $k.Substring(0, 12)
    rc = $rc
    accounts_json = $count
    status = $status
    log = $log
  }
}

$out | Format-Table -AutoSize
$out | ConvertTo-Json -Depth 3 | Out-File -Encoding UTF8 tmp_key_test_result.json
Write-Output "RESULT_JSON=tmp_key_test_result.json"
