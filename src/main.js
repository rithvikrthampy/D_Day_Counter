// Retrieve Tauri Window API if available
let appWindow = null;
if (window.__TAURI__ && window.__TAURI__.window) {
  appWindow = window.__TAURI__.window.getCurrentWindow();
}

// Audio Synthesizer using Web Audio API
const AudioSynth = {
  ctx: null,
  enabled: false,

  init() {
    if (!this.ctx) {
      this.ctx = new (window.AudioContext || window.webkitAudioContext)();
    }
  },

  playTick() {
    if (!this.enabled) return;
    try {
      this.init();
      const osc = this.ctx.createOscillator();
      const gain = this.ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(1400, this.ctx.currentTime);
      osc.frequency.exponentialRampToValueAtTime(700, this.ctx.currentTime + 0.04);
      gain.gain.setValueAtTime(0.015, this.ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + 0.04);
      osc.connect(gain);
      gain.connect(this.ctx.destination);
      osc.start();
      osc.stop(this.ctx.currentTime + 0.04);
    } catch (e) {
      console.warn("Audio Synth fail:", e);
    }
  },

  playClick() {
    if (!this.enabled) return;
    try {
      this.init();
      const osc = this.ctx.createOscillator();
      const gain = this.ctx.createGain();
      osc.type = 'square';
      osc.frequency.setValueAtTime(900, this.ctx.currentTime);
      gain.gain.setValueAtTime(0.01, this.ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + 0.03);
      osc.connect(gain);
      gain.connect(this.ctx.destination);
      osc.start();
      osc.stop(this.ctx.currentTime + 0.03);
    } catch (e) {
      console.warn("Audio Synth fail:", e);
    }
  },

  playAlarm() {
    if (!this.enabled) return;
    try {
      this.init();
      const now = this.ctx.currentTime;
      for (let i = 0; i < 4; i++) {
        const start = now + i * 0.4;
        const osc = this.ctx.createOscillator();
        const gain = this.ctx.createGain();
        osc.type = 'sawtooth';
        osc.frequency.setValueAtTime(980, start);
        gain.gain.setValueAtTime(0.03, start);
        gain.gain.exponentialRampToValueAtTime(0.001, start + 0.25);
        osc.connect(gain);
        gain.connect(this.ctx.destination);
        osc.start(start);
        osc.stop(start + 0.25);
      }
    } catch (e) {
      console.warn("Audio Synth fail:", e);
    }
  }
};

// Preset state
let presets = [];
let currentPresetIndex = -1;

// General State variables
let eventName = "MISSION SEQUENCE";
let targetDate = "";
let creationDate = "";
let themeClass = "theme-cyan";
let alwaysOnTop = false;
let showCrt = true;
let showGrid = true;
let widgetOpacity = 100;
let lastSec = -1;
let notificationSent = false;

// DOM Elements
let widgetContainer;
let eventNameDisplay;
let daysVal, hoursVal, minutesVal, secondsVal;
let settingsPanel;

// Form Inputs
let inputEventName;
let inputTargetDate;
let themeOpts;
let inputAlwaysOnTop;
let inputAutostart;
let inputCrt;
let inputGrid;
let inputSound;
let presetsList;
let inputOpacity;
let opacityValDisplay;
let autostart = false;

// Load state from localStorage or set defaults
function loadState() {
  const isInitialized = localStorage.getItem("chrono_initialized_v2") === "true";
  if (!isInitialized) {
    localStorage.setItem("chrono_sound_enabled", "false");
    localStorage.setItem("chrono_initialized_v2", "true");
  }

  eventName = localStorage.getItem("chrono_event_name") || "LAUNCH SEQUENCE";
  themeClass = localStorage.getItem("chrono_theme") || "theme-cyan";
  alwaysOnTop = localStorage.getItem("chrono_always_on_top") === "true";
  
  showCrt = localStorage.getItem("chrono_crt_enabled") !== "false";
  showGrid = localStorage.getItem("chrono_grid_enabled") !== "false";
  AudioSynth.enabled = localStorage.getItem("chrono_sound_enabled") === "true";
  widgetOpacity = parseInt(localStorage.getItem("chrono_opacity") || "100", 10);

  // Load presets database
  try {
    const rawPresets = localStorage.getItem("chrono_presets");
    presets = rawPresets ? JSON.parse(rawPresets) : [
      { name: "LAUNCH SEQUENCE", target: new Date(Date.now() + 300000000).toISOString().slice(0, 16) },
      { name: "TACTICAL EXAM", target: new Date(Date.now() + 150000000).toISOString().slice(0, 16) }
    ];
  } catch (e) {
    presets = [];
  }

  const savedTarget = localStorage.getItem("chrono_target_date");
  const savedCreation = localStorage.getItem("chrono_creation_date");
  const now = new Date();
  
  if (savedTarget) {
    targetDate = savedTarget;
  } else if (presets.length > 0) {
    targetDate = presets[0].target;
    eventName = presets[0].name;
    currentPresetIndex = 0;
  } else {
    const defaultTarget = new Date(now.getTime() + (3 * 24 + 12) * 60 * 60 * 1000);
    targetDate = defaultTarget.toISOString().slice(0, 16);
  }
  
  if (savedCreation) {
    creationDate = savedCreation;
  } else {
    creationDate = now.toISOString();
  }

  // Find index of current preset
  currentPresetIndex = presets.findIndex(p => p.name === eventName && p.target === targetDate);
}

// Save state to localStorage
function saveState() {
  localStorage.setItem("chrono_event_name", eventName);
  localStorage.setItem("chrono_target_date", targetDate);
  localStorage.setItem("chrono_creation_date", creationDate);
  localStorage.setItem("chrono_theme", themeClass);
  localStorage.setItem("chrono_always_on_top", alwaysOnTop ? "true" : "false");
  localStorage.setItem("chrono_crt_enabled", showCrt ? "true" : "false");
  localStorage.setItem("chrono_grid_enabled", showGrid ? "true" : "false");
  localStorage.setItem("chrono_sound_enabled", AudioSynth.enabled ? "true" : "false");
  localStorage.setItem("chrono_opacity", widgetOpacity);
  localStorage.setItem("chrono_presets", JSON.stringify(presets));
}

// Apply visual themes and window options
function applyConfigurations() {
  if (widgetContainer) {
    // Classes for Theme, Grid and Scanline toggles
    let classes = `widget-container ${themeClass}`;
    if (!showGrid) classes += " grid-disabled";
    if (!showCrt) classes += " scanlines-disabled";
    widgetContainer.className = classes;
    widgetContainer.style.opacity = widgetOpacity / 100;
  }
  
  if (appWindow) {
    appWindow.setAlwaysOnTop(alwaysOnTop).catch(err => {
      console.error("Failed to set always on top:", err);
    });
  }
}

// Alarm tracking to prevent playing it repeatedly
let alarmPlayed = false;

// Update the countdown display
function updateCountdown() {
  const now = new Date().getTime();
  const target = new Date(targetDate).getTime();
  
  const totalDiff = target - now;
  
  // Progress Bar elements
  const progressBar = document.getElementById("progress-fill-bar");
  const progressPctVal = document.getElementById("progress-pct-val");
  
  // Track seconds for ticking
  const currentSec = Math.floor(now / 1000);
  let secondsTicked = false;
  if (currentSec !== lastSec) {
    lastSec = currentSec;
    secondsTicked = true;
  }
  
  if (totalDiff <= 0) {
    // Post-D-day Elapsed mode (Count up)
    const elapsed = Math.abs(totalDiff);
    const days = Math.floor(elapsed / (1000 * 60 * 60 * 24));
    const hours = Math.floor((elapsed % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
    const minutes = Math.floor((elapsed % (1000 * 60 * 60)) / (1000 * 60));
    const seconds = Math.floor((elapsed % (1000 * 60)) / 1000);

    daysVal.textContent = "+" + String(days).padStart(2, '0');
    hoursVal.textContent = String(hours).padStart(2, '0');
    minutesVal.textContent = String(minutes).padStart(2, '0');
    secondsVal.textContent = String(seconds).padStart(2, '0');
    
    eventNameDisplay.textContent = `${eventName} // OVERDUE`;
    
    eventNameDisplay.style.color = "var(--danger-color)";
    eventNameDisplay.style.borderColor = "var(--danger-color)";
    eventNameDisplay.style.textShadow = "0 0 8px var(--danger-color)";

    // Progress is 0 when overdue
    if (progressBar) progressBar.style.width = "0%";
    if (progressPctVal) progressPctVal.textContent = "0.0%";

    // Send OS Notification
    if (window.__TAURI__ && window.__TAURI__.notification && !notificationSent) {
      window.__TAURI__.notification.sendNotification({
        title: "CHRONO TARGET REACHED",
        body: `The countdown for "${eventName}" has completed!`
      }).catch(err => console.error("Notification trigger fail:", err));
      notificationSent = true;
    }

    // Play terminal alarm once
    if (!alarmPlayed) {
      AudioSynth.playAlarm();
      alarmPlayed = true;
    }
    return;
  }
  
  alarmPlayed = false;
  eventNameDisplay.style.color = "";
  eventNameDisplay.style.borderColor = "";
  eventNameDisplay.style.textShadow = "";
  eventNameDisplay.textContent = eventName;
  
  const days = Math.floor(totalDiff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((totalDiff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const minutes = Math.floor((totalDiff % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = Math.floor((totalDiff % (1000 * 60)) / 1000);
  
  daysVal.textContent = String(days).padStart(2, '0');
  hoursVal.textContent = String(hours).padStart(2, '0');
  minutesVal.textContent = String(minutes).padStart(2, '0');
  secondsVal.textContent = String(seconds).padStart(2, '0');

  // Compute progress percentage
  const creationTime = new Date(creationDate).getTime();
  const targetTime = target;
  const totalDuration = targetTime - creationTime;
  const timePassed = now - creationTime;
  let percentage = 100;
  if (totalDuration > 0 && timePassed >= 0) {
    percentage = 100 - (timePassed / totalDuration) * 100;
    if (percentage < 0) percentage = 0;
    if (percentage > 100) percentage = 100;
  }
  if (progressBar) progressBar.style.width = percentage.toFixed(1) + "%";
  if (progressPctVal) progressPctVal.textContent = percentage.toFixed(1) + "%";

  // Play subtle ticking sound on seconds transition
  if (secondsTicked) {
    AudioSynth.playTick();
  }
}

// Preset management helpers
function renderPresets() {
  if (!presetsList) return;
  presetsList.innerHTML = "";
  
  presets.forEach((preset, idx) => {
    const div = document.createElement("div");
    div.className = "preset-item" + (idx === currentPresetIndex ? " active" : "");
    
    div.innerHTML = `
      <span class="preset-name-lbl">${preset.name}</span>
      <div class="preset-actions">
        <button class="preset-btn select-btn" data-idx="${idx}">LOAD</button>
        <button class="preset-btn del-btn" data-idx="${idx}">×</button>
      </div>
    `;
    presetsList.appendChild(div);
  });

  // Attach button events
  presetsList.querySelectorAll(".select-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      AudioSynth.playClick();
      const idx = parseInt(btn.getAttribute("data-idx"));
      loadPreset(idx);
    });
  });

  presetsList.querySelectorAll(".del-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      AudioSynth.playClick();
      const idx = parseInt(btn.getAttribute("data-idx"));
      deletePreset(idx);
    });
  });
}

function loadPreset(idx) {
  if (idx < 0 || idx >= presets.length) return;
  currentPresetIndex = idx;
  eventName = presets[idx].name;
  targetDate = presets[idx].target;
  creationDate = new Date().toISOString();
  notificationSent = false;
  saveState();
  updateCountdown();
  renderPresets();
  
  // Update Settings Form Inputs in case settings are currently open
  if (inputEventName) inputEventName.value = eventName;
  if (inputTargetDate) inputTargetDate.value = targetDate;
}

function deletePreset(idx) {
  presets.splice(idx, 1);
  if (currentPresetIndex === idx) {
    currentPresetIndex = presets.length > 0 ? 0 : -1;
    if (currentPresetIndex !== -1) {
      eventName = presets[0].name;
      targetDate = presets[0].target;
    }
  } else if (currentPresetIndex > idx) {
    currentPresetIndex--;
  }
  saveState();
  updateCountdown();
  renderPresets();
}

// scrolling console logs
const consoleLines = [
  "SYS.STATUS: OPERATION LOG ONLINE",
  "GRID.ENCRYPTION: SHIELD MAXIMUM",
  "CHRONO.QUANTUM: SYNC COMPLETED",
  "NEON.REACTOR: STABLE AT 100%",
  "TEMP.CORE: NORMAL RANGE (32C)",
  "SIGNAL.TAC: SECURE BEACON COMM",
  "MEM.SECTOR: SCRUBBING CACHE",
  "FIRMWARE.INTEGRITY: SECURE",
  "AUTOSTART.RUNNER: WAITING",
  "HUD.MATRIX: CHROMATIC BUFF"
];

function triggerLogConsole() {
  const consoleEl = document.getElementById("tactical-console");
  if (!consoleEl) return;
  
  // Pick random log
  const text = consoleLines[Math.floor(Math.random() * consoleLines.length)];
  const div = document.createElement("div");
  div.className = "console-line";
  div.textContent = `> ${text}`;
  
  consoleEl.appendChild(div);
  
  // Keep max 3 lines
  while (consoleEl.children.length > 3) {
    consoleEl.removeChild(consoleEl.firstChild);
  }
}

// Initialize on DOM Load
window.addEventListener("DOMContentLoaded", () => {
  // Select DOM Elements
  widgetContainer = document.getElementById("widget-container");
  eventNameDisplay = document.getElementById("event-name-display");
  daysVal = document.getElementById("days-val");
  hoursVal = document.getElementById("hours-val");
  minutesVal = document.getElementById("minutes-val");
  secondsVal = document.getElementById("seconds-val");
  settingsPanel = document.getElementById("settings-panel");
  
  inputEventName = document.getElementById("input-event-name");
  inputTargetDate = document.getElementById("input-target-date");
  themeOpts = document.querySelectorAll(".theme-opt");
  inputAlwaysOnTop = document.getElementById("input-always-on-top");
  inputAutostart = document.getElementById("input-autostart");
  
  // Custom Customization settings
  inputCrt = document.getElementById("input-crt");
  inputGrid = document.getElementById("input-grid");
  inputSound = document.getElementById("input-sound");
  presetsList = document.getElementById("presets-list");
  inputOpacity = document.getElementById("input-opacity");
  opacityValDisplay = document.getElementById("opacity-val-display");

  if (inputOpacity) {
    inputOpacity.addEventListener("input", () => {
      const val = inputOpacity.value;
      if (opacityValDisplay) opacityValDisplay.textContent = val + "%";
      if (widgetContainer) widgetContainer.style.opacity = val / 100;
    });
  }

  // Request system notification permissions on load
  if (window.__TAURI__ && window.__TAURI__.notification) {
    window.__TAURI__.notification.isPermissionGranted().then(granted => {
      if (!granted) {
        window.__TAURI__.notification.requestPermission();
      }
    });
  }

  // Query current autostart status from Registry on load
  if (window.__TAURI__ && window.__TAURI__.core) {
    const { invoke } = window.__TAURI__.core;
    invoke("is_autostart_enabled")
      .then(enabled => {
        autostart = enabled;
        if (inputAutostart) inputAutostart.checked = autostart;
      })
      .catch(err => console.error("Error reading autostart status: ", err));
  }
  
  // Load local settings
  loadState();
  applyConfigurations();
  
  // Start the countdown update loop via requestAnimationFrame
  updateCountdown();
  function tick() {
    updateCountdown();
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);

  // Set up console log transitions
  setInterval(triggerLogConsole, 6000);
  
  // Make window visible once loaded and size state plugin is ready to prevent window flashing
  if (appWindow) {
    appWindow.show().catch(err => console.error("Failed to show window:", err));
  }

  // Set up event listeners for window actions
  document.getElementById("btn-minimize").addEventListener("click", () => {
    AudioSynth.playClick();
    if (appWindow) {
      appWindow.hide();
    }
  });
  
  document.getElementById("btn-close").addEventListener("click", () => {
    AudioSynth.playClick();
    if (appWindow) {
      appWindow.hide();
    }
  });
  
  // Preset switcher arrows on the front widget face
  document.getElementById("btn-prev-preset").addEventListener("click", () => {
    AudioSynth.playClick();
    if (presets.length <= 1) return;
    let newIdx = currentPresetIndex - 1;
    if (newIdx < 0) newIdx = presets.length - 1;
    loadPreset(newIdx);
  });

  document.getElementById("btn-next-preset").addEventListener("click", () => {
    AudioSynth.playClick();
    if (presets.length <= 1) return;
    let newIdx = currentPresetIndex + 1;
    if (newIdx >= presets.length) newIdx = 0;
    loadPreset(newIdx);
  });

  // Settings menu buttons
  document.getElementById("btn-settings").addEventListener("click", () => {
    AudioSynth.playClick();
    // Open settings panel and load state values into form inputs
    inputEventName.value = eventName;
    inputTargetDate.value = targetDate;
    inputAlwaysOnTop.checked = alwaysOnTop;
    if (inputAutostart) inputAutostart.checked = autostart;
    
    // Custom switches
    if (inputCrt) inputCrt.checked = showCrt;
    if (inputGrid) inputGrid.checked = showGrid;
    if (inputSound) inputSound.checked = AudioSynth.enabled;
    if (inputOpacity) {
      inputOpacity.value = widgetOpacity;
      if (opacityValDisplay) opacityValDisplay.textContent = widgetOpacity + "%";
    }

    renderPresets();

    themeOpts.forEach(opt => {
      if (opt.getAttribute("data-theme") === themeClass) {
        opt.classList.add("active");
      } else {
        opt.classList.remove("active");
      }
    });
    
    settingsPanel.classList.add("open");
  });
  
  document.getElementById("btn-close-settings").addEventListener("click", () => {
    AudioSynth.playClick();
    settingsPanel.classList.remove("open");
  });

  inputTargetDate.addEventListener("click", () => {
    try {
      inputTargetDate.showPicker();
    } catch (err) {
      console.error("showPicker API not supported: ", err);
    }
  });
  
  themeOpts.forEach(btn => {
    btn.addEventListener("click", () => {
      AudioSynth.playClick();
      themeOpts.forEach(o => o.classList.remove("active"));
      btn.classList.add("active");
    });
  });

  // Presets Add Timer Button click
  const btnAddPreset = document.getElementById("btn-add-preset");
  if (btnAddPreset) {
    btnAddPreset.addEventListener("click", () => {
      AudioSynth.playClick();
      const defaultTarget = new Date(Date.now() + 3 * 24 * 60 * 60 * 1000).toISOString().slice(0, 16);
      presets.push({ name: "NEW EVENT", target: defaultTarget });
      currentPresetIndex = presets.length - 1;
      
      // Load this preset instantly
      loadPreset(currentPresetIndex);
    });
  }

  // Commit settings changes
  document.getElementById("btn-save-settings").addEventListener("click", () => {
    AudioSynth.playClick();
    const newEventName = inputEventName.value.trim() || "SYS.EVENT";
    const newTargetDate = inputTargetDate.value;
    
    if (!newTargetDate) {
      alert("PLEASE CHOOSE A VALID TARGET TIMESTAMP.");
      return;
    }
    
    const activeThemeBtn = document.querySelector(".theme-opt.active");
    const newTheme = activeThemeBtn ? activeThemeBtn.getAttribute("data-theme") : "theme-cyan";
    const newAlwaysOnTop = inputAlwaysOnTop.checked;
    const newAutostart = inputAutostart ? inputAutostart.checked : false;

    // Custom toggles
    const newCrt = inputCrt ? inputCrt.checked : true;
    const newGrid = inputGrid ? inputGrid.checked : true;
    const newSound = inputSound ? inputSound.checked : true;
    const newOpacity = inputOpacity ? parseInt(inputOpacity.value, 10) : 100;

    if (newAutostart !== autostart && window.__TAURI__ && window.__TAURI__.core) {
      const { invoke } = window.__TAURI__.core;
      invoke("set_autostart", { enable: newAutostart })
        .then(() => {
          autostart = newAutostart;
        })
        .catch(err => {
          console.error("Failed to commit autostart configuration: ", err);
          alert("FAILED TO SET AUTOSTART REGISTRY VALUE.");
        });
    }
    
    eventName = newEventName;
    targetDate = newTargetDate;
    creationDate = new Date().toISOString();
    themeClass = newTheme;
    alwaysOnTop = newAlwaysOnTop;
    showCrt = newCrt;
    showGrid = newGrid;
    AudioSynth.enabled = newSound;
    widgetOpacity = newOpacity;
    notificationSent = false;
    
    // Save current active event back into the preset list to synchronize it
    if (currentPresetIndex !== -1 && currentPresetIndex < presets.length) {
      presets[currentPresetIndex].name = eventName;
      presets[currentPresetIndex].target = targetDate;
    }

    saveState();
    applyConfigurations();
    updateCountdown();
    
    settingsPanel.classList.remove("open");
  });

  // ==================== AUTO UPDATER CLIENT ====================
  if (window.__TAURI__ && window.__TAURI__.core) {
    const { invoke } = window.__TAURI__.core;
    
    let detectedUpdateVersion = null;

    // Function to run update check
    function checkUpdates(isManual = false) {
      const statusEl = document.getElementById("manual-check-status");
      const btnManual = document.getElementById("btn-check-updates-manual");
      
      if (isManual) {
        if (statusEl) statusEl.textContent = "QUERYING SERVERS...";
        if (btnManual) {
          btnManual.disabled = true;
          btnManual.textContent = "CHECKING...";
        }
      }
      
      invoke("check_for_updates")
        .then(newVersion => {
          if (newVersion) {
            detectedUpdateVersion = newVersion;
            
            // Show update indicator on settings gear button
            const gearBtn = document.getElementById("btn-settings");
            if (gearBtn) gearBtn.classList.add("has-update");
            
            // Show update banner inside settings panel
            const banner = document.getElementById("update-banner");
            const bannerVer = document.getElementById("update-banner-ver");
            if (banner) banner.style.display = "flex";
            if (bannerVer) bannerVer.textContent = "v" + newVersion;

            // Push a warning log line to the console
            const consoleEl = document.getElementById("tactical-console");
            if (consoleEl) {
              const div = document.createElement("div");
              div.className = "console-line";
              div.textContent = `> [WARN] NEW FIRMWARE v${newVersion} DETECTED.`;
              consoleEl.appendChild(div);
              while (consoleEl.children.length > 3) {
                consoleEl.removeChild(consoleEl.firstChild);
              }
            }

            // Play alert sound
            AudioSynth.playAlarm();

            // Set the version string in the update modal
            const verVal = document.getElementById("update-version-val");
            if (verVal) verVal.textContent = "v" + newVersion;

            // If manual, open the confirmation modal directly
            if (isManual) {
              const prompt = document.getElementById("update-prompt");
              if (prompt) prompt.classList.add("open");
            }
          } else {
            if (isManual && statusEl) {
              statusEl.textContent = "FIRMWARE IS UP TO DATE.";
              setTimeout(() => { statusEl.textContent = ""; }, 3000);
            }
          }
        })
        .catch(err => {
          console.error("Error checking for updates:", err);
          if (isManual && statusEl) {
            statusEl.textContent = "CONNECTION DRIFT / ERROR.";
            setTimeout(() => { statusEl.textContent = ""; }, 3000);
          }
        })
        .finally(() => {
          if (isManual && btnManual) {
            btnManual.disabled = false;
            btnManual.textContent = "CHECK FOR UPDATES";
          }
        });
    }

    // Check for updates automatically shortly after startup
    setTimeout(() => {
      checkUpdates(false);
    }, 4000);

    // Check periodically every 24 hours while open
    setInterval(() => {
      checkUpdates(false);
    }, 86400000);

    // Hook manual check button
    const btnManualCheck = document.getElementById("btn-check-updates-manual");
    if (btnManualCheck) {
      btnManualCheck.addEventListener("click", () => {
        AudioSynth.playClick();
        checkUpdates(true);
      });
    }

    // Hook banner install button to open the modal
    const btnBannerInstall = document.getElementById("btn-banner-update");
    if (btnBannerInstall) {
      btnBannerInstall.addEventListener("click", () => {
        AudioSynth.playClick();
        const prompt = document.getElementById("update-prompt");
        const verVal = document.getElementById("update-version-val");
        if (prompt) {
          if (verVal) verVal.textContent = detectedUpdateVersion ? "v" + detectedUpdateVersion : "NEW VERSION";
          prompt.classList.add("open");
        }
      });
    }

    const btnConfirmUpdate = document.getElementById("btn-confirm-update");
    const btnCancelUpdate = document.getElementById("btn-cancel-update");
    
    if (btnConfirmUpdate) {
      btnConfirmUpdate.addEventListener("click", () => {
        AudioSynth.playClick();
        btnConfirmUpdate.textContent = "DOWNLOADING...";
        btnConfirmUpdate.disabled = true;
        if (btnCancelUpdate) btnCancelUpdate.disabled = true;

        invoke("start_update_install")
          .catch(err => {
            console.error("Update install failed:", err);
            alert("FIRMWARE DOWNLOAD FAILED.");
            btnConfirmUpdate.textContent = "COMMIT UPDATE";
            btnConfirmUpdate.disabled = false;
            if (btnCancelUpdate) btnCancelUpdate.disabled = false;
          });
      });
    }

    if (btnCancelUpdate) {
      btnCancelUpdate.addEventListener("click", () => {
        AudioSynth.playClick();
        const prompt = document.getElementById("update-prompt");
        if (prompt) prompt.classList.remove("open");
      });
    }
  }
});
