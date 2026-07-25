#!/usr/bin/env node
// ==============================================================================
// COPYRIGHT & PATENT NOTICE
// Copyright (c) 2026 Samay Mehar (ID: 487266569521). All Rights Reserved.
//
// PATENTS PENDING & INTELLECTUAL PROPERTY NOTICE:
// This source code, algorithm, architecture, and underlying inventions are the
// proprietary intellectual property of Samay Mehar (ID: 487266569521).
// Protected under national and international copyright, patent, and trade secret laws.
//
// COMMERCIAL LICENSE NOTICE:
// Unauthorized copying, modification, distribution, reverse engineering, or commercial
// exploitation of this software in whole or in part without express written authorization
// from Samay Mehar is strictly prohibited.
// ==============================================================================
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

// Determine the binary path based on the OS
const ext = process.platform === 'win32' ? '.exe' : '';
const binName = `sap-mcp-server${ext}`;
const workspaceRoot = path.join(__dirname, '..', '..');
const debugBinPath = path.join(workspaceRoot, 'target', 'debug', binName);
const releaseBinPath = path.join(workspaceRoot, 'target', 'release', binName);

let targetBin = null;

// Mock check for downloaded prod binary (e.g. in ./bin)
const prodBinPath = path.join(__dirname, binName);
if (fs.existsSync(prodBinPath)) {
    targetBin = prodBinPath;
} else if (fs.existsSync(releaseBinPath)) {
    targetBin = releaseBinPath;
} else if (fs.existsSync(debugBinPath)) {
    targetBin = debugBinPath;
}

if (!targetBin) {
    console.error(`[sapiola-mcp] Binary not found.`);
    console.error(`[sapiola-mcp] Please run 'npm install' to fetch or build it.`);
    process.exit(1);
}

if (targetBin === releaseBinPath || targetBin === debugBinPath) {
    console.error(`\n[sapiola-mcp] ⚠️ RUNNING LOCAL FALLBACK BINARY: ${targetBin} ⚠️\n`);
}

spawnSync(targetBin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(0);
