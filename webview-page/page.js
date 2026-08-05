// crest-synth MIXER projection page (crest-spec asset.WebviewProjectionPage).
//
// PURE RENDER. `render(model)` rebuilds the five shell bands from one
// deserialized SemanticGraphicalViewModel document and nothing else: no
// Date.now, no Math.random, no accumulated state, no incidental
// iteration-order dependence (every walk follows the document's own array
// order). The same document always paints the same DOM.
//
// The one carve-out is presentation-only meter animation (crest-spec C-002):
// the crest://meters listener repaints ONLY the meter element from the
// latest AudioObservationSnapshot frame, mirroring the eframe rule — a
// reading shows only when the frame's parameterGeneration and
// activeGraphRevision both match the document on screen; a missing or stale
// frame reads the zero state.
//
// This page registers no key handler and captures no input; keys are
// captured Rust-side (WP01/WP02 boundary). Listener glue (tauri events →
// parse → render) is kept separate from the pure function so the WP06
// harness can drive `render`/`renderObservation` headlessly.
"use strict";

(function pageModule() {
  var PROJECTION_EVENT = "crest://projection";
  var METER_EVENT = "crest://meters";
  var PAINTED_EVENT = "crest://painted";

  // Explicit-unavailability mark: a declared structure with no view data
  // behind it says so; it is never painted with a representative value and
  // never dressed as the resting state (MixerTrackColumnStructure rule).
  var UNAVAILABLE = "UNAVAILABLE";

  // The declared column anatomy, closed and ordered (crest-spec
  // valueObject.MixerTrackColumnStructure). Rendering walks exactly this
  // list; the observation reports it per column so a lost or reordered
  // structure is measurable.
  var COLUMN_ANATOMY = [
    "TrackHeader",
    "LevelFader",
    "LevelReadout",
    "PanReadout",
    "StateLine",
  ];

  // ---- pure helpers ------------------------------------------------------

  function escapeHtml(text) {
    return String(text).replace(/[&<>"']/g, function (ch) {
      return {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      }[ch];
    });
  }

  // Unwraps the nested value shapes the document serializes:
  // {kind:"scalar", value:0.5} and {kind:"parameter", value:{kind:"toggle",
  // value:false}} both resolve to their innermost primitive.
  function innerValue(control) {
    var value = control && control.value;
    var guard = 0;
    while (value !== null && typeof value === "object" && guard < 8) {
      value = value.value;
      guard += 1;
    }
    return value;
  }

  function toggleOn(control) {
    var value = innerValue(control);
    return value === true || value === 1;
  }

  // Normalized position of a continuous control inside its declared range.
  function fraction(control) {
    var range = control && control.numericRange;
    var value = innerValue(control);
    if (!range || typeof value !== "number" || range.maximum === range.minimum) {
      return 0;
    }
    var f = (value - range.minimum) / (range.maximum - range.minimum);
    return Math.min(1, Math.max(0, f));
  }

  // The bound MidiHexadecimal form: (v-min)/(max-min)*127 as two uppercase
  // hex digits. Bound to the LevelReadout presentation (and the Inspector's
  // reading of the same focused level) — crest-spec ValuePresentationForm.
  function midiHex(control) {
    var scaled = Math.round(fraction(control) * 127);
    var hex = scaled.toString(16).toUpperCase();
    return hex.length < 2 ? "0" + hex : hex;
  }

  function midiDecimal(control) {
    return String(Math.round(fraction(control) * 127));
  }

  // The authored pan label-and-condition pair; a centered pan is a presented
  // value ("C"), never an absence.
  function panCondition(control) {
    var f = fraction(control);
    if (f < 0.45) {
      return "L";
    }
    return f > 0.55 ? "R" : "C";
  }

  function trackName(trackId) {
    var hex = Number(trackId).toString(16).toUpperCase();
    return "T" + (hex.length < 2 ? "0" + hex : hex);
  }

  function surfaceById(model, id) {
    var surfaces = model.surfaces || [];
    for (var i = 0; i < surfaces.length; i += 1) {
      if (surfaces[i].id === id) {
        return surfaces[i];
      }
    }
    return null;
  }

  // Groups the flat mixer-main control list into track columns keyed by
  // track id, preserving the document's declared first-appearance order.
  function trackColumns(mainSurface) {
    var order = [];
    var byTrack = {};
    var controls = (mainSurface && mainSurface.controls) || [];
    for (var i = 0; i < controls.length; i += 1) {
      var control = controls[i];
      var id = control.path.controlId.id;
      if (!id || id.kind !== "track") {
        continue;
      }
      if (!Object.prototype.hasOwnProperty.call(byTrack, id.track_id)) {
        byTrack[id.track_id] = { trackId: id.track_id };
        order.push(byTrack[id.track_id]);
      }
      byTrack[id.track_id][id.parameter] = control;
    }
    return order;
  }

  function columnFocused(column) {
    var parameters = ["level", "pan", "mute", "solo"];
    for (var i = 0; i < parameters.length; i += 1) {
      var control = column[parameters[i]];
      if (control && control.focused) {
        return true;
      }
    }
    return false;
  }

  // Fill/emphasis state for the column's fader, resolved in declared
  // priority: focus, then error, then disabled, then mute, then solo, then
  // rest. Each is also carried by text or shape elsewhere in the column, so
  // color is never the only signal.
  function faderState(column) {
    if (columnFocused(column)) {
      return "focused";
    }
    var level = column.level;
    if (level && level.error) {
      return "error";
    }
    if (level && level.enabled === false) {
      return "disabled";
    }
    if (column.mute && toggleOn(column.mute)) {
      return "muted";
    }
    if (column.solo && toggleOn(column.solo)) {
      return "soloed";
    }
    return "resting";
  }

  // The focused control anywhere in the document, in surface order.
  function focusedControl(model) {
    var surfaces = model.surfaces || [];
    for (var s = 0; s < surfaces.length; s += 1) {
      var controls = surfaces[s].controls || [];
      for (var c = 0; c < controls.length; c += 1) {
        if (controls[c].focused) {
          return controls[c];
        }
      }
    }
    return null;
  }

  // Cursor identity, e.g. "T00 / LEVEL", derived from the focused control's
  // own label ("T00 Level"); falls back to the focus path's identity parts.
  function focusIdentity(model) {
    var control = focusedControl(model);
    if (control && control.label) {
      var label = String(control.label);
      var match = label.match(/^(T[0-9A-Fa-f]{2})\s+(.+)$/);
      if (match) {
        return match[1].toUpperCase() + " / " + match[2].toUpperCase();
      }
      return label.toUpperCase();
    }
    var id =
      model.focusPath && model.focusPath.controlId && model.focusPath.controlId.id;
    if (id && id.kind === "track") {
      return trackName(id.track_id) + " / " + String(id.parameter).toUpperCase();
    }
    return "";
  }

  // The focused mixer track id, or null — the meter rule and the Inspector
  // both key on it.
  function focusedTrackId(model) {
    var id =
      model.focusPath && model.focusPath.controlId && model.focusPath.controlId.id;
    return id && id.kind === "track" ? id.track_id : null;
  }

  function statusToneClass(status) {
    if (!status) {
      return "muted";
    }
    if (status.kind === "ready") {
      return "positive";
    }
    return status.kind === "error" ? "warning" : "adjust";
  }

  // Condenses a valid action's label to the authored hint form:
  // "Move right" -> "right", "Open PATCH" -> "patch", "Adjust mode" -> "adjust".
  function hintLabel(action) {
    return String(action.label)
      .toLowerCase()
      .replace(/^(open|move)\s+/, "")
      .replace(/\s+mode$/, "");
  }

  function hintRun(actions) {
    var html = "";
    for (var i = 0; i < actions.length; i += 1) {
      var action = actions[i];
      if (!action.hint) {
        continue; // null hints never render (spike defect, kept fixed)
      }
      html +=
        '<span class="type-hint focus">' +
        escapeHtml(action.hint) +
        ":" +
        escapeHtml(hintLabel(action)) +
        "</span>";
    }
    return html;
  }

  function actionsOfKind(model, kind) {
    var out = [];
    var actions = model.validActions || [];
    for (var i = 0; i < actions.length; i += 1) {
      if (actions[i].action && actions[i].action.kind === kind) {
        out.push(actions[i]);
      }
    }
    return out;
  }

  // ---- band renderers (pure: document in, HTML string out) ---------------

  function contextLineHtml(model) {
    // The two declared top-level contexts; the active one carries the
    // authored "*" marker so activity is legible beyond color.
    var contexts = ["patch", "mixer"];
    var run = "";
    for (var i = 0; i < contexts.length; i += 1) {
      var name = contexts[i];
      var active = model.context === name;
      run +=
        '<span class="type-label context-entry' +
        (active ? " focus" : " muted") +
        '" data-context="' +
        name +
        '"' +
        (active ? ' data-active="true"' : "") +
        ">" +
        (active ? '<span class="patch">*</span> ' : "") +
        name.toUpperCase() +
        "</span>";
    }
    var status = model.status || null;
    var statusHtml = status
      ? '<span class="type-label ' +
        statusToneClass(status) +
        '" data-role="status">' +
        escapeHtml(status.label) +
        "</span>"
      : "";
    return (
      '<span class="type-heading">CREST SYNTH</span>' +
      '<span class="spring"></span>' +
      run +
      statusHtml
    );
  }

  function identityHeaderHtml(model, columns) {
    return (
      '<span class="type-display">' +
      escapeHtml(String(model.context || "").toUpperCase()) +
      "</span>" +
      '<span class="type-label muted">/ ' +
      columns.length +
      " TRACKS</span>" +
      '<span class="spring"></span>' +
      '<span class="type-value focus" data-role="focus-annotation">' +
      escapeHtml(focusIdentity(model)) +
      "</span>"
    );
  }

  function columnHtml(column) {
    var focused = columnFocused(column);
    var state = faderState(column);
    var header =
      '<span class="structure track-header type-label ' +
      (focused ? "focus" : "secondary") +
      '" data-structure="TrackHeader">' +
      trackName(column.trackId) +
      "</span>";

    var fader;
    if (column.level) {
      var level = fraction(column.level).toFixed(6);
      fader =
        '<div class="structure level-fader" data-structure="LevelFader" data-state="' +
        state +
        '" style="--level:' +
        level +
        '">' +
        '<div class="fader-track"><div class="fader-fill"></div>' +
        '<div class="fader-cap"></div></div></div>';
    } else {
      fader =
        '<div class="structure level-fader unavailable" data-structure="LevelFader"' +
        ' data-state="unavailable"><span class="type-hint muted">' +
        UNAVAILABLE +
        "</span></div>";
    }

    var readout = column.level
      ? '<span class="structure level-readout type-value ' +
        (focused ? "focus" : "patch") +
        '" data-structure="LevelReadout">' +
        midiHex(column.level) +
        "</span>"
      : '<span class="structure level-readout type-value muted unavailable"' +
        ' data-structure="LevelReadout">' +
        UNAVAILABLE +
        "</span>";

    var pan = column.pan
      ? '<span class="structure pan-readout type-hint" data-structure="PanReadout">' +
        '<span class="muted">P</span> <span class="secondary">' +
        panCondition(column.pan) +
        "</span></span>"
      : '<span class="structure pan-readout type-hint muted unavailable"' +
        ' data-structure="PanReadout">P ' +
        UNAVAILABLE +
        "</span>";

    // Mute and solo together on one line, resting condition included, so an
    // engaged pair is never reduced to whichever was set last and a resting
    // line never reads as an unavailable one.
    var stateLine;
    if (column.mute || column.solo) {
      var mutePart = column.mute
        ? toggleOn(column.mute)
          ? '<span class="warning">M ON</span>'
          : '<span class="muted">M --</span>'
        : '<span class="muted">M ' + UNAVAILABLE + "</span>";
      var soloPart = column.solo
        ? toggleOn(column.solo)
          ? '<span class="positive">S ON</span>'
          : '<span class="muted">S --</span>'
        : '<span class="muted">S ' + UNAVAILABLE + "</span>";
      // The separator carries no surrounding spaces: the authored "M --" /
      // "S ON" marks plus a bare interpunct measure 72 px in the hint
      // style, which is what lets the line hold single-line inside the
      // authored 82 px desktop column (the spaced form measures 88 px and
      // cannot).
      stateLine =
        '<span class="structure state-line type-hint" data-structure="StateLine">' +
        mutePart +
        '<span class="muted">·</span>' +
        soloPart +
        "</span>";
    } else {
      stateLine =
        '<span class="structure state-line type-hint muted unavailable"' +
        ' data-structure="StateLine">M/S ' +
        UNAVAILABLE +
        "</span>";
    }

    return (
      '<div class="column' +
      (focused ? " focused" : "") +
      '" data-track="' +
      column.trackId +
      '">' +
      header +
      fader +
      readout +
      pan +
      stateLine +
      "</div>"
    );
  }

  function workspaceHtml(model, columns) {
    var navigate = actionsOfKind(model, "navigate");
    var mode = actionsOfKind(model, "setInteractionMode");
    var bank;
    if (columns.length > 0) {
      var cells = "";
      for (var i = 0; i < columns.length; i += 1) {
        cells += columnHtml(columns[i]);
      }
      bank = '<div class="bank" id="bank">' + cells + "</div>";
    } else {
      // A bank with no track view data marks the bank unavailable rather
      // than painting a representative mixer (MixerStripBank rule).
      bank =
        '<div class="bank bank-unavailable" id="bank">' +
        '<span class="type-label muted">MIXER BANK ' +
        UNAVAILABLE +
        "</span></div>";
    }
    return (
      '<div class="caption-row">' +
      '<span class="type-label muted">LEVEL / PAN / MUTE / SOLO</span>' +
      // The focused track's meter reading, centered on the legend line the
      // way the eframe adapter paints it; render always writes the
      // zero/stale state and only the meter listener may overwrite it.
      '<span class="type-value secondary meter" id="meter-readout">' +
      (focusedTrackId(model) === null ? "" : "METER 0.000") +
      "</span>" +
      '<span class="spring"></span>' +
      hintRun(navigate) +
      "</div>" +
      bank +
      '<div class="hint-row">' +
      hintRun(mode) +
      '<span class="spring"></span>' +
      "</div>"
    );
  }

  function inspectorHtml(model) {
    var inspector = surfaceById(model, "mixerInspector");
    var main = surfaceById(model, "mixerMain");
    if (!inspector) {
      // The persistent side region with no surface behind it says so; it is
      // never painted with representative content.
      return (
        '<span class="type-label muted">CURSOR</span>' +
        '<span class="type-value muted" data-role="cursor">' +
        UNAVAILABLE +
        "</span>"
      );
    }

    var trackId = focusedTrackId(model);
    var focused = focusedControl(model);
    var columns = trackColumns(main);
    var column = null;
    for (var i = 0; i < columns.length; i += 1) {
      if (columns[i].trackId === trackId) {
        column = columns[i];
      }
    }

    // Big readout: the focused control's value in its bound presentation
    // form — the focused level reads in the LevelReadout's MidiHexadecimal
    // binding; a toggle reads ON/OFF; any other continuous reads 0–127.
    var big = UNAVAILABLE;
    var bigTone = "muted";
    var rangeHint = "";
    if (focused) {
      var parameter =
        focused.path.controlId.id && focused.path.controlId.id.parameter;
      if (focused.kind === "toggle") {
        big = toggleOn(focused) ? "ON" : "OFF";
        bigTone = toggleOn(focused) ? "positive" : "secondary";
      } else if (parameter === "level") {
        big = midiHex(focused);
        bigTone = "positive";
        rangeHint = "LEVEL 00 — 7F";
      } else if (parameter === "pan") {
        big = panCondition(focused);
        bigTone = "positive";
        rangeHint = "PAN L — C — R";
      } else if (focused.numericRange) {
        big = midiDecimal(focused);
        bigTone = "positive";
        rangeHint = "0 — 127";
      }
    }

    var muteSolo = "";
    if (column && (column.mute || column.solo)) {
      var muteValue = column.mute
        ? toggleOn(column.mute)
          ? '<span class="warning">ON</span>'
          : "OFF"
        : UNAVAILABLE;
      var soloValue = column.solo
        ? toggleOn(column.solo)
          ? '<span class="positive">ON</span>'
          : "OFF"
        : UNAVAILABLE;
      muteSolo =
        '<div class="rule"></div>' +
        '<table class="type-hint secondary" data-role="mute-solo">' +
        "<tbody>" +
        '<tr><td>MUTE</td><td data-role="mute">' +
        muteValue +
        "</td></tr>" +
        '<tr><td>SOLO</td><td data-role="solo">' +
        soloValue +
        "</td></tr>" +
        "</tbody></table>";
    }

    // The focused track's sends, in the surface's declared order.
    var sendRows = "";
    var controls = inspector.controls || [];
    for (var c = 0; c < controls.length; c += 1) {
      var control = controls[c];
      var id = control.path.controlId.id;
      if (!id || id.kind !== "send" || id.track_id !== trackId) {
        continue;
      }
      var label = String(control.label)
        .replace(/^T[0-9A-Fa-f]{2}\s+/, "")
        .toUpperCase();
      sendRows +=
        '<tr data-send="' +
        id.bus +
        '"><td>' +
        escapeHtml(label) +
        "</td><td>" +
        midiDecimal(control) +
        "</td></tr>";
    }
    var sends = sendRows
      ? '<div class="rule"></div>' +
        '<table class="type-hint secondary" data-role="sends"><tbody>' +
        sendRows +
        "</tbody></table>"
      : "";

    return (
      '<span class="type-label muted">CURSOR</span>' +
      '<span class="type-value focus" data-role="cursor">' +
      escapeHtml(focusIdentity(model)) +
      "</span>" +
      '<span class="type-display big-readout ' +
      bigTone +
      '" data-role="big-readout">' +
      escapeHtml(big) +
      "</span>" +
      (rangeHint
        ? '<span class="type-hint muted" data-role="readout-range">' +
          escapeHtml(rangeHint) +
          "</span>"
        : "") +
      muteSolo +
      sends
    );
  }

  function footerHtml(model) {
    var breadcrumb = String(model.context || "").toUpperCase();
    var identity = focusIdentity(model);
    if (identity) {
      breadcrumb += " / " + identity;
    }
    return (
      '<span class="type-hint secondary" data-role="breadcrumb">' +
      escapeHtml(breadcrumb) +
      "</span>" +
      '<span class="spring"></span>' +
      hintRun(model.validActions || [])
    );
  }

  // ---- the pure render ---------------------------------------------------

  // Rebuilds the five shell bands from one deserialized
  // SemanticGraphicalViewModel document. Same document, identical DOM.
  function render(model) {
    var doc = window.document;
    var main = surfaceById(model, "mixerMain");
    var columns = trackColumns(main);
    doc.getElementById("context-line").innerHTML = contextLineHtml(model);
    doc.getElementById("identity-header").innerHTML = identityHeaderHtml(
      model,
      columns
    );
    doc.getElementById("workspace").innerHTML = workspaceHtml(model, columns);
    doc.getElementById("inspector").innerHTML = inspectorHtml(model);
    doc.getElementById("footer").innerHTML = footerHtml(model);
  }

  // ---- the structural observation (WP06 T024 contract) -------------------

  // Renders the document, then reads the painted DOM back into post-paint
  // structural evidence. Everything in the returned object is copied from
  // what was actually painted — not from the input document — so a plan that
  // never reached the screen cannot satisfy it. Shape:
  //
  //   {
  //     generation, stateHash,            // echoed document identity
  //     bands: { contextLine, identityHeader, workspace, inspector,
  //              footer },                // true = present with painted area
  //     columns: [{ trackId, header, structures: [..in painted order..],
  //                 focused, levelHex, pan, stateLine }],
  //     focus: { target, trackId },       // painted cursor identity
  //     inspector: { widthPx,             // computed width, whole pixels
  //                  cursor, bigReadout, mute, solo,
  //                  sends: [{ label, value }] },  // painted declared order
  //     meter: "METER 0.000"              // the painted meter text
  //   }
  //
  // Two calls with one document at one window size return deep-equal
  // objects; inspector.widthPx is asserted >= 320 at the compact viewport.
  function renderObservation(model) {
    render(model);
    var doc = window.document;

    function painted(id) {
      var el = doc.getElementById(id);
      if (!el) {
        return false;
      }
      var rect = el.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    }

    function textOf(root, selector) {
      var el = root.querySelector(selector);
      return el ? el.textContent.replace(/\s+/g, " ").trim() : null;
    }

    var bands = {
      contextLine: painted("context-line"),
      identityHeader: painted("identity-header"),
      workspace: painted("workspace"),
      inspector: painted("inspector"),
      footer: painted("footer"),
    };

    var columns = [];
    var columnNodes = doc.querySelectorAll("#bank .column");
    for (var i = 0; i < columnNodes.length; i += 1) {
      var node = columnNodes[i];
      var structures = [];
      var structureNodes = node.querySelectorAll("[data-structure]");
      for (var s = 0; s < structureNodes.length; s += 1) {
        structures.push(structureNodes[s].getAttribute("data-structure"));
      }
      columns.push({
        trackId: Number(node.getAttribute("data-track")),
        header: textOf(node, '[data-structure="TrackHeader"]'),
        structures: structures,
        focused: node.classList.contains("focused"),
        levelHex: textOf(node, '[data-structure="LevelReadout"]'),
        pan: textOf(node, '[data-structure="PanReadout"]'),
        stateLine: textOf(node, '[data-structure="StateLine"]'),
      });
    }

    var inspectorElement = doc.getElementById("inspector");
    var sends = [];
    var sendRows = inspectorElement.querySelectorAll(
      '[data-role="sends"] tr[data-send]'
    );
    for (var r = 0; r < sendRows.length; r += 1) {
      var cells = sendRows[r].querySelectorAll("td");
      sends.push({
        label: cells[0].textContent.trim(),
        value: cells[1].textContent.trim(),
      });
    }

    var focusedNode = doc.querySelector("#bank .column.focused");
    return {
      generation: model.generation,
      stateHash: model.stateHash,
      bands: bands,
      columns: columns,
      focus: {
        target: textOf(inspectorElement, '[data-role="cursor"]'),
        trackId: focusedNode
          ? Number(focusedNode.getAttribute("data-track"))
          : null,
      },
      inspector: {
        widthPx: Math.round(inspectorElement.getBoundingClientRect().width),
        cursor: textOf(inspectorElement, '[data-role="cursor"]'),
        bigReadout: textOf(inspectorElement, '[data-role="big-readout"]'),
        mute: textOf(inspectorElement, '[data-role="mute"]'),
        solo: textOf(inspectorElement, '[data-role="solo"]'),
        sends: sends,
      },
      meter: textOf(doc, "#meter-readout"),
      anatomy: COLUMN_ANATOMY.slice(),
    };
  }

  // ---- listener glue (kept apart from the pure render) -------------------

  // Presentation-only meter animation state: the document on screen and the
  // latest snapshot frame. Never read by render().
  var latestModel = null;
  var latestFrame = null;

  function graphRevisionsMatch(frame, model) {
    return (
      JSON.stringify(frame.activeGraphRevision) ===
      JSON.stringify(model.status && model.status.graphRevision)
    );
  }

  // Repaints only the meter element: the focused track's rms when the frame
  // matches the document on screen (same parameterGeneration, same graph
  // revision), the zero state otherwise — the eframe stale rule, verbatim.
  function updateMeter() {
    var el = window.document.getElementById("meter-readout");
    if (!el || !latestModel) {
      return;
    }
    var trackId = focusedTrackId(latestModel);
    if (trackId === null) {
      el.textContent = "";
      return;
    }
    var rms = 0;
    if (
      latestFrame &&
      latestFrame.parameterGeneration === latestModel.generation &&
      graphRevisionsMatch(latestFrame, latestModel) &&
      latestFrame.tracks &&
      latestFrame.tracks[trackId]
    ) {
      rms = latestFrame.tracks[trackId].rms;
    }
    el.textContent = "METER " + Number(rms).toFixed(3);
  }

  // Post-paint evidence for the crest://painted ack: the rendered
  // generation plus each declared shell region's painted bounds and first
  // visible label, copied from the DOM after the frame painted — the
  // region ids spell ShellRegionId's serialized names so the Rust side can
  // assemble honest ShellFrameObservation values without re-deriving them.
  function paintedEvidence(generation) {
    var doc = window.document;
    var bands = [
      { id: "contextLine", element: "context-line" },
      { id: "identityHeader", element: "identity-header" },
      { id: "mainWorkspace", element: "workspace" },
      { id: "persistentSideRegion", element: "inspector" },
      { id: "footer", element: "footer" },
    ];
    var regions = [];
    for (var i = 0; i < bands.length; i += 1) {
      var el = doc.getElementById(bands[i].element);
      if (!el) {
        continue;
      }
      var rect = el.getBoundingClientRect();
      var label = "";
      var nodes = el.querySelectorAll("span");
      for (var n = 0; n < nodes.length; n += 1) {
        var text = nodes[n].textContent.replace(/\s+/g, " ").trim();
        if (text) {
          label = text;
          break;
        }
      }
      regions.push({
        id: bands[i].id,
        xPx: rect.x,
        yPx: rect.y,
        widthPx: rect.width,
        heightPx: rect.height,
        label: label,
      });
    }
    return {
      generation: generation,
      viewport: {
        widthPx: window.innerWidth,
        heightPx: window.innerHeight,
      },
      regions: regions,
    };
  }

  function attachTransports() {
    var tauri = window.__TAURI__;
    if (!tauri || !tauri.event) {
      return; // headless harness drives window.crest.render directly
    }
    tauri.event.listen(PROJECTION_EVENT, function (event) {
      latestModel = event.payload;
      render(latestModel);
      updateMeter();
      // Paint ack: echo the rendered generation (with post-paint region
      // evidence) once this frame has painted, so the harness can measure
      // projection-to-paint latency and correlate painted regions.
      var generation = latestModel.generation;
      window.requestAnimationFrame(function () {
        tauri.event.emit(PAINTED_EVENT, paintedEvidence(generation));
      });
    });
    tauri.event.listen(METER_EVENT, function (event) {
      latestFrame = event.payload;
      updateMeter();
    });
  }

  window.crest = { render: render, renderObservation: renderObservation };
  attachTransports();
})();
