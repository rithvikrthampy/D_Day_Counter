// Retrieve Tauri Window API if available
let appWindow = null;
if (window.__TAURI__ && window.__TAURI__.window) {
  appWindow = window.__TAURI__.window.getCurrentWindow();
}

// State variables
let eventName = "MISSION SEQUENCE";
let targetDate = "";
let creationDate = "";
let themeClass = "theme-cyan";
let alwaysOnTop = false;

// DOM Elements
let widgetContainer;
let eventNameDisplay;
let daysVal, hoursVal, minutesVal, secondsVal;
let progressPctVal, progressFillBar;
let settingsPanel;

// Form Inputs
let inputEventName;
let inputTargetDate;
let themeOpts;
let inputAlwaysOnTop;
let inputAutostart;
let autostart = false;

// Load state from localStorage or set defaults
function loadState() {
  eventName = localStorage.getItem("chrono_event_name") || "LAUNCH SEQUENCE";
  themeClass = localStorage.getItem("chrono_theme") || "theme-cyan";
  alwaysOnTop = localStorage.getItem("chrono_always_on_top") === "true";
  
  const savedTarget = localStorage.getItem("chrono_target_date");
  const savedCreation = localStorage.getItem("chrono_creation_date");
  
  const now = new Date();
  if (savedTarget) {
    targetDate = savedTarget;
  } else {
    // Default to 3 days and 12 hours from now
    const defaultTarget = new Date(now.getTime() + (3 * 24 + 12) * 60 * 60 * 1000);
    targetDate = defaultTarget.toISOString().slice(0, 16); // format to datetime-local string (YYYY-MM-DDTHH:MM)
  }
  
  if (savedCreation) {
    creationDate = savedCreation;
  } else {
    creationDate = now.toISOString();
  }
}

// Save state to localStorage
function saveState() {
  localStorage.setItem("chrono_event_name", eventName);
  localStorage.setItem("chrono_target_date", targetDate);
  localStorage.setItem("chrono_creation_date", creationDate);
  localStorage.setItem("chrono_theme", themeClass);
  localStorage.setItem("chrono_always_on_top", alwaysOnTop ? "true" : "false");
}

// Apply visual themes and window options
function applyConfigurations() {
  // Apply theme class to widget container
  if (widgetContainer) {
    widgetContainer.className = `widget-container ${themeClass}`;
  }
  
  // Apply window always on top configuration
  if (appWindow) {
    appWindow.setAlwaysOnTop(alwaysOnTop).catch(err => {
      console.error("Failed to set always on top:", err);
    });
  }
}

// Update the countdown display
function updateCountdown() {
  const now = new Date().getTime();
  const target = new Date(targetDate).getTime();
  const start = new Date(creationDate).getTime();
  
  const totalDiff = target - now;
  
  if (totalDiff <= 0) {
    // Timer expired
    daysVal.textContent = "00";
    hoursVal.textContent = "00";
    minutesVal.textContent = "00";
    secondsVal.textContent = "00";
    
    eventNameDisplay.textContent = `${eventName} // TERMINAL`;
    progressPctVal.textContent = "0.00%";
    progressFillBar.style.width = "0%";
    
    // Add warning styling to the container or text when expired
    eventNameDisplay.style.color = "var(--danger-color)";
    eventNameDisplay.style.borderColor = "var(--danger-color)";
    eventNameDisplay.style.textShadow = "0 0 8px var(--danger-color)";
    return;
  }
  
  // Reset normal theme colors on event name in case it was previously expired
  eventNameDisplay.style.color = "";
  eventNameDisplay.style.borderColor = "";
  eventNameDisplay.style.textShadow = "";
  eventNameDisplay.textContent = eventName;
  
  // Calculations
  const days = Math.floor(totalDiff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((totalDiff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const minutes = Math.floor((totalDiff % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = Math.floor((totalDiff % (1000 * 60)) / 1000);
  
  // Update UI values
  daysVal.textContent = String(days).padStart(2, '0');
  hoursVal.textContent = String(hours).padStart(2, '0');
  minutesVal.textContent = String(minutes).padStart(2, '0');
  secondsVal.textContent = String(seconds).padStart(2, '0');
  
  // Calculate percentage remaining
  const totalDuration = target - start;
  let pct = 0;
  if (totalDuration > 0) {
    const elapsed = now - start;
    pct = Math.max(0, Math.min(100, (1 - (elapsed / totalDuration)) * 100));
  }
  
  progressPctVal.textContent = `${pct.toFixed(2)}%`;
  progressFillBar.style.width = `${pct}%`;
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
  progressPctVal = document.getElementById("progress-pct-val");
  progressFillBar = document.getElementById("progress-fill-bar");
  settingsPanel = document.getElementById("settings-panel");
  
  inputEventName = document.getElementById("input-event-name");
  inputTargetDate = document.getElementById("input-target-date");
  themeOpts = document.querySelectorAll(".theme-opt");
  inputAlwaysOnTop = document.getElementById("input-always-on-top");
  inputAutostart = document.getElementById("input-autostart");

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
  
  // Start the countdown update loop (every 1s)
  updateCountdown();
  const timerInterval = setInterval(updateCountdown, 1000);
  
  // Set up event listeners for window actions
  document.getElementById("btn-minimize").addEventListener("click", () => {
    if (appWindow) {
      appWindow.minimize();
    } else {
      console.log("Mock Minimize");
    }
  });
  
  document.getElementById("btn-close").addEventListener("click", () => {
    if (appWindow) {
      appWindow.close();
    } else {
      console.log("Mock Close");
    }
  });
  
  // Settings menu buttons
  document.getElementById("btn-settings").addEventListener("click", () => {
    // Open settings panel and load state values into form inputs
    inputEventName.value = eventName;
    inputTargetDate.value = targetDate;
    inputAlwaysOnTop.checked = alwaysOnTop;
    if (inputAutostart) inputAutostart.checked = autostart;
    
    // Set active theme button selection
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
    settingsPanel.classList.remove("open");
  });

  // Automatically trigger calendar & time picker when the input is clicked
  inputTargetDate.addEventListener("click", () => {
    try {
      inputTargetDate.showPicker();
    } catch (err) {
      console.error("showPicker API not supported: ", err);
    }
  });
  
  // Theme selector click handling
  themeOpts.forEach(btn => {
    btn.addEventListener("click", () => {
      themeOpts.forEach(o => o.classList.remove("active"));
      btn.classList.add("active");
    });
  });
  
  // Commit settings changes
  document.getElementById("btn-save-settings").addEventListener("click", () => {
    // Read input values
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

    // Toggle registry entry if the autostart option was changed
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
    
    // Update State
    eventName = newEventName;
    targetDate = newTargetDate;
    creationDate = new Date().toISOString(); // Reset start reference on change
    themeClass = newTheme;
    alwaysOnTop = newAlwaysOnTop;
    
    // Persist and Apply changes
    saveState();
    applyConfigurations();
    
    // Refresh countdown UI immediately
    updateCountdown();
    
    // Hide panel
    settingsPanel.classList.remove("open");
  });
});
