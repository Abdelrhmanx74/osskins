# Party Mode Test Runner with formatted output

Write-Host "`n=== Running Party Mode Tests ===" -ForegroundColor Cyan
Write-Host "Please wait...`n" -ForegroundColor Yellow

# Run tests and capture output
$output = cargo test --lib -- party_mode --nocapture 2>&1 | Out-String

# Parse the output
$lines = $output -split "`n"
$passed = @()
$failed = @()
$inFailureDetails = $false
$currentFailure = ""
$failureDetails = @{}

foreach ($line in $lines) {
    # Capture test results
    if ($line -match "^test .* \.\.\. ok$") {
        $testName = ($line -replace "^test ", "" -replace " \.\.\. ok$", "").Trim()
        $passed += $testName
    }
    elseif ($line -match "^test .* \.\.\. FAILED$") {
        $testName = ($line -replace "^test ", "" -replace " \.\.\. FAILED$", "").Trim()
        $failed += $testName
    }
    # Capture failure details
    elseif ($line -match "^---- .* stdout ----$") {
        $inFailureDetails = $true
        $currentFailure = ($line -replace "^---- ", "" -replace " stdout ----$", "").Trim()
        $failureDetails[$currentFailure] = @()
    }
    elseif ($inFailureDetails -and $line -match "^----") {
        $inFailureDetails = $false
        $currentFailure = ""
    }
    elseif ($inFailureDetails -and $currentFailure -ne "" -and $line.Trim() -ne "") {
        $failureDetails[$currentFailure] += $line
    }
}

# Display summary
Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor White
Write-Host "║           PARTY MODE TEST SUMMARY                          ║" -ForegroundColor White
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor White

$total = $passed.Count + $failed.Count
$passRate = if ($total -gt 0) { [math]::Round(($passed.Count / $total) * 100, 1) } else { 0 }

Write-Host "`nTotal Tests: $total" -ForegroundColor Cyan
Write-Host "✓ Passed: $($passed.Count) ($passRate%)" -ForegroundColor Green
Write-Host "✗ Failed: $($failed.Count)" -ForegroundColor Red

# Show failed tests with details
if ($failed.Count -gt 0) {
    Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║           FAILED TESTS DETAILS                             ║" -ForegroundColor Red
    Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Red
    
    $index = 1
    foreach ($test in $failed) {
        Write-Host "`n[$index/$($failed.Count)] " -NoNewline -ForegroundColor Yellow
        Write-Host "$($test.Split('::')[-1])" -ForegroundColor Red
        Write-Host "  Module: $($test -replace '::[^:]+$', '')" -ForegroundColor DarkGray
        
        # Show failure reason
        if ($failureDetails.ContainsKey($test)) {
            $details = $failureDetails[$test] | Where-Object { $_ -match "(assertion|panicked|left|right)" }
            foreach ($detail in $details) {
                if ($detail -match "left:") {
                    Write-Host "    Actual:   " -NoNewline -ForegroundColor Yellow
                    Write-Host ($detail -replace "^\s*left:\s*", "") -ForegroundColor White
                }
                elseif ($detail -match "right:") {
                    Write-Host "    Expected: " -NoNewline -ForegroundColor Yellow
                    Write-Host ($detail -replace "^\s*right:\s*", "") -ForegroundColor White
                }
                elseif ($detail -match "assertion") {
                    Write-Host "    Error: $($detail.Trim())" -ForegroundColor Red
                }
            }
        }
        $index++
    }
}

# Show test categories summary
Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           TEST CATEGORIES                                   ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

$categories = @{
    "Helper Tests" = "test_helpers"
    "Timing Edge Cases" = "test_timing_edge_cases"
    "ARAM Mode" = "test_aram_mode"
    "Swift Play" = "test_swift_play"
    "Session State" = "test_session_state"
    "Party Detection" = "test_party_detection"
    "Injection Logic" = "test_injection_logic"
    "Race Conditions" = "test_race_conditions"
}

foreach ($category in $categories.GetEnumerator() | Sort-Object Name) {
    $catPassed = ($passed | Where-Object { $_ -match $category.Value }).Count
    $catFailed = ($failed | Where-Object { $_ -match $category.Value }).Count
    $catTotal = $catPassed + $catFailed
    
    if ($catTotal -gt 0) {
        $status = if ($catFailed -eq 0) { "✓" } else { "✗" }
        $color = if ($catFailed -eq 0) { "Green" } else { "Yellow" }
        Write-Host "$status $($category.Key): " -NoNewline -ForegroundColor $color
        Write-Host "$catPassed/$catTotal passed" -ForegroundColor White
    }
}

Write-Host ""

# Return exit code
if ($failed.Count -gt 0) {
    exit 1
} else {
    exit 0
}
