// crest-synth projection page (crest-spec asset.WebviewProjectionPage).
//
// PURE RENDER. `render(model)` rebuilds the five shell bands from one
// deserialized SemanticGraphicalViewModel document and nothing else: no
// Date.now, no Math.random, no accumulated state, no incidental
// iteration-order dependence (every walk follows the document's own array
// order). The same document always paints the same DOM. Both declared
// top-level contexts render here — MIXER as the sixteen-column strip bank,
// PATCH as the listed strip of projected rows — through the same shared
// structural bands (context line, identity header, workspace scaffold,
// persistent side region, footer); nothing forks the schema and no field is
// invented (crest-spec requirement.serialized_projection_transport).
//
// The one carve-out is presentation-only meter animation (crest-spec C-002):
// the crest://meters listener repaints ONLY the meter element from the
// latest AudioObservationSnapshot frame, mirroring the retired-shell rule — a
// reading shows only when the frame's parameterGeneration and
// activeGraphRevision both match the document on screen; a missing or stale
// frame reads the zero state.
//
// This page registers no key handler and captures no input; keys are
// captured Rust-side (WP01/WP02 boundary). Listener glue (tauri events →
// parse → render → paint ack) is kept separate from the pure function so the
// acceptance harness can drive `render`/`renderObservation` headlessly.
"use strict";

(function pageModule() {
  var PROJECTION_EVENT = "crest://projection";
  var METER_EVENT = "crest://meters";
  var PAINTED_EVENT = "crest://painted";
  var RENDER_ERROR_EVENT = "crest://render-error";

  // Explicit-unavailability mark: a declared structure with no view data
  // behind it says so; it is never painted with a representative value and
  // never dressed as the resting state (MixerTrackColumnStructure rule).
  var UNAVAILABLE = "UNAVAILABLE";

  // The row-level unavailable mark the pre-cutover shipped rows painted
  // (parameter_row::UNAVAILABLE_MARK): a designed row whose value data did
  // not arrive reads "--" where its value would be.
  var UNAVAILABLE_MARK = "--";

  // The authored hint separator (primitives::hint::HINT_SEPARATOR).
  var HINT_SEPARATOR = " · ";

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

  // The five entries the PATCH Utility panel is designed to carry, in
  // authored order (utility_inspector_panel::DESIGNED_UTILITY_ENTRIES).
  // `driver: null` means the projection carries no identity for the entry at
  // all; it is marked explicitly unavailable rather than omitted or invented.
  var DESIGNED_UTILITY_ENTRIES = [
    { label: "MASTER VOLUME", driver: null },
    { label: "PATCH VOLUME", driver: "patch.output.trimGainDb" },
    { label: "MIDI INPUT", driver: null },
    { label: "OUTPUT TRACK", driver: "patch.output.outputTrack" },
    { label: "VOICE LIMIT", driver: null },
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

  // Where a row's value sits inside its declared bounds, or null when the
  // document carries no bounds or no numeric value — a row drawn half-full
  // because nobody said how full it is would be a claim the projection never
  // made (parameter_row::position_fraction).
  function positionFraction(control) {
    var range = control && control.numericRange;
    var value = innerValue(control);
    if (!range || typeof value !== "number" || range.maximum === range.minimum) {
      return null;
    }
    return fraction(control);
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

  // The document's persistent side surface — the Inspector on MIXER, the
  // Utility region on PATCH — resolved by declared role, not by context fork.
  function persistentSideSurface(model) {
    var surfaces = model.surfaces || [];
    for (var i = 0; i < surfaces.length; i += 1) {
      if (surfaces[i].role === "persistentSide") {
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

  // ---- the shared control-state vocabulary --------------------------------

  // Derives the declared ComponentState treatment for one projected control
  // from the immutable document plus the projected interaction mode —
  // exactly the precedence the pre-cutover shipped row applied
  // (patch_strip_row::component_state): a failed edit outranks an in-flight
  // one, focus outranks read-only-ness. Matching is exhaustive over what the
  // document can carry; an unknown lifecycle kind or interaction mode is an
  // explicit visible `?state` marker, never a silent resting row.
  function controlState(control, mode) {
    if (control.error) {
      return { name: "error", raw: null };
    }
    var status = control.status || null;
    if (status) {
      var kind = String(status.kind);
      if (kind === "preparing" || kind === "activating") {
        return { name: "loading", raw: null };
      }
      if (kind !== "ready" && kind !== "failed") {
        return { name: "unknown", raw: kind };
      }
    }
    if (control.focused) {
      var modeName = String(mode);
      if (modeName === "adjust") {
        return { name: "adjusting", raw: null };
      }
      if (
        modeName === "navigate" ||
        modeName === "modal" ||
        modeName === "multiSelect"
      ) {
        return { name: "focused", raw: null };
      }
      return { name: "unknown", raw: modeName };
    }
    if (control.enabled && control.editable) {
      return { name: "resting", raw: null };
    }
    return { name: "disabled", raw: null };
  }

  // Renders one typed document value as finished text, mirroring the shipped
  // row's presentation (parameter_row::value_text): continuous values read to
  // three places, toggles read ON/OFF, identities and summaries read as
  // themselves, assets read their locator. An unknown value kind is an
  // explicit `?kind` marker, never an empty cell.
  function controlValueText(control) {
    var value = control && control.value;
    if (!value || typeof value !== "object") {
      return UNAVAILABLE_MARK;
    }
    if (value.kind === "scalar") {
      return Number(value.value).toFixed(3);
    }
    if (value.kind === "parameter") {
      var parameter = value.value;
      if (parameter && typeof parameter === "object") {
        if (parameter.kind === "continuous") {
          return Number(parameter.value).toFixed(3);
        }
        if (parameter.kind === "stepped") {
          return String(parameter.value);
        }
        if (parameter.kind === "choice") {
          return String(parameter.value);
        }
        if (parameter.kind === "toggle") {
          return parameter.value ? "ON" : "OFF";
        }
        return "?" + String(parameter.kind);
      }
      return UNAVAILABLE_MARK;
    }
    if (value.kind === "asset") {
      return value.value && value.value.locator
        ? String(value.value.locator)
        : UNAVAILABLE_MARK;
    }
    if (value.kind === "identity" || value.kind === "summary") {
      return String(value.value);
    }
    return "?" + String(value.kind);
  }

  // The non-color signal a state carries beside its row (state.rs
  // NonColorSignal): Disabled says "Locked", Loading says the document's own
  // lifecycle word, Error says the typed failure text, an unknown state says
  // its explicit `?state` marker. Shape-signalled states return no text.
  function stateMarkHtml(control, state) {
    if (state.name === "disabled") {
      return '<span class="muted">Locked</span>';
    }
    if (state.name === "loading") {
      var word = control.status ? String(control.status.label) : "";
      return '<span class="adjust">' + escapeHtml(word) + "</span>";
    }
    if (state.name === "error") {
      var failure = control.error ? String(control.error.label) : "";
      return '<span class="warning">' + escapeHtml(failure) + "</span>";
    }
    if (state.name === "unknown") {
      return '<span class="warning">?' + escapeHtml(String(state.raw)) + "</span>";
    }
    return "";
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

  // The identity band, one shared skeleton for both contexts: the context
  // display, a projected metadata run, and the focus annotation. Only the
  // metadata content differs, and it comes from the document's own surface
  // summaries, never from a page-side fork.
  function identityHeaderHtml(model, columns) {
    var metadata;
    if (model.context === "patch") {
      var main = surfaceById(model, "patchMain");
      var summary = (main && main.summary) || null;
      metadata =
        summary && summary.kind === "patch"
          ? escapeHtml(String(summary.patch_name)) +
            HINT_SEPARATOR +
            escapeHtml(String(summary.capability_id))
          : UNAVAILABLE_MARK;
    } else {
      metadata = columns.length + " TRACKS";
    }
    return (
      '<span class="type-display">' +
      escapeHtml(String(model.context || "").toUpperCase()) +
      "</span>" +
      '<span class="type-label muted">/ ' +
      metadata +
      "</span>" +
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
        '" data-level="' +
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

  // The meter element every workspace carries: render always writes the
  // zero/stale state (or nothing when no mixer track is focused) and only
  // the meter listener may overwrite it.
  function meterHtml(model) {
    return (
      '<span class="type-value secondary meter" id="meter-readout">' +
      (focusedTrackId(model) === null ? "" : "METER 0.000") +
      "</span>"
    );
  }

  // The shared workspace scaffold both contexts ride: one caption row
  // (context-specific left run, the meter, a spring, a context-specific
  // right run, the navigate hints), the workspace body, and one mode hint
  // row. The rows are structural bands, not per-context forks.
  function workspaceScaffold(model, leftHtml, rightHtml, bodyHtml) {
    var navigate = actionsOfKind(model, "navigate");
    var mode = actionsOfKind(model, "setInteractionMode");
    return (
      '<div class="caption-row">' +
      leftHtml +
      meterHtml(model) +
      '<span class="spring"></span>' +
      rightHtml +
      hintRun(navigate) +
      "</div>" +
      bodyHtml +
      '<div class="hint-row">' +
      hintRun(mode) +
      '<span class="spring"></span>' +
      "</div>"
    );
  }

  function mixerWorkspaceHtml(model, columns) {
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
    return workspaceScaffold(
      model,
      '<span class="type-label muted">LEVEL / PAN / MUTE / SOLO</span>',
      "",
      bank
    );
  }

  // ---- the PATCH strip ----------------------------------------------------

  // One projected control as a listed strip row: label left, the state's
  // non-color mark or the position indicator in the middle, the rendered
  // value (and unit) right — the shipped parameter-row anatomy. The derived
  // state rides the row as data plus a class so every treatment resolves
  // from the token vocabulary in page.css.
  function patchRowHtml(control, mode, role) {
    var state = controlState(control, mode);
    var id = String(
      control.path && control.path.controlId ? control.path.controlId.id : ""
    );
    var mark = stateMarkHtml(control, state);
    var middle;
    if (mark) {
      middle =
        '<span class="prow-mark type-hint" data-role="state-mark">' +
        mark +
        "</span>" +
        '<span class="spring"></span>';
    } else {
      var position = positionFraction(control);
      middle =
        position === null
          ? '<span class="spring"></span>'
          : '<div class="prow-position"><div class="prow-position-fill"' +
            ' data-position="' +
            position.toFixed(6) +
            '"></div></div>';
    }
    var unit = control.unit
      ? '<span class="prow-unit type-hint muted">' +
        escapeHtml(String(control.unit)) +
        "</span>"
      : "";
    var row =
      '<div class="prow' +
      (role === "panel" ? " panel" : "") +
      '" data-control="' +
      escapeHtml(id) +
      '" data-state="' +
      state.name +
      '">' +
      '<span class="prow-label type-label">' +
      escapeHtml(String(control.label)) +
      "</span>" +
      middle +
      '<span class="prow-value type-value">' +
      escapeHtml(controlValueText(control)) +
      "</span>" +
      unit +
      "</div>";
    return row + lifecycleHtml(control, id);
  }

  // The lifecycle band beneath a row reporting a structural edit (error or
  // in-flight status): the typed failure and/or lifecycle word, the active
  // and requested graph revisions, and the explicitly unavailable requested
  // value — the shipped row's readout, verbatim
  // (patch_strip_row::render_lifecycle). It takes no focus and appears only
  // while the document reports the edit.
  function lifecycleHtml(control, id) {
    var status = control.status || null;
    var inFlight =
      status && (status.kind === "preparing" || status.kind === "activating");
    if (!control.error && !inFlight) {
      return "";
    }
    var parts = [];
    if (control.error) {
      parts.push(String(control.error.label));
    }
    if (status) {
      parts.push(String(status.label));
      parts.push("ACTIVE GRAPH");
      parts.push(String(status.graphRevision));
      parts.push("REQUESTED GRAPH");
      parts.push(
        status.targetGraphRevision === null ||
          status.targetGraphRevision === undefined
          ? UNAVAILABLE_MARK
          : String(status.targetGraphRevision)
      );
    }
    // Not projected. Marked rather than filled, so an absent requested value
    // is never mistaken for the active one under another name.
    parts.push("REQUESTED VALUE");
    parts.push(UNAVAILABLE_MARK);
    var tone = control.error ? "warning" : "adjust";
    return (
      '<div class="lifecycle type-hint ' +
      tone +
      '" data-lifecycle-for="' +
      escapeHtml(id) +
      '">' +
      escapeHtml(parts.join(HINT_SEPARATOR)) +
      "</div>"
    );
  }

  // A designed structure the projection supplied no entry for: its static
  // name stays on screen and the authored row mark sits where its value
  // would be (section::mark_unavailable).
  function markUnavailableRowHtml(structure) {
    return (
      '<div class="prow unavailable" data-state="disabled">' +
      '<span class="prow-label type-label muted">' +
      escapeHtml(structure) +
      "</span>" +
      '<span class="spring"></span>' +
      '<span class="prow-value type-value muted">' +
      UNAVAILABLE_MARK +
      "</span>" +
      "</div>"
    );
  }

  // The PATCH main workspace: the section header content (the surface's own
  // label left, the focused entry's annotation right) on the shared caption
  // row, then every visible projected control as one strip row in the
  // document's declared order. The page inserts, reorders, and hides
  // nothing; the reducer owns the PATCH focus order.
  function patchWorkspaceHtml(model) {
    var main = surfaceById(model, "patchMain");
    var controls = (main && main.controls) || [];
    var mode = model.interactionMode;
    var rows = "";
    var arranged = 0;
    for (var i = 0; i < controls.length; i += 1) {
      if (!controls[i].visible) {
        continue;
      }
      rows += patchRowHtml(controls[i], mode, "listed");
      arranged += 1;
    }
    var body =
      arranged > 0
        ? '<div class="strip" id="strip">' + rows + "</div>"
        : '<div class="strip strip-unavailable" id="strip">' +
          markUnavailableRowHtml("ENTRIES") +
          "</div>";
    var focused = null;
    for (var f = 0; f < controls.length; f += 1) {
      if (controls[f].focused) {
        focused = controls[f];
        break;
      }
    }
    var annotation = focused
      ? '<span class="type-hint focus" data-role="section-annotation">FOCUS' +
        HINT_SEPARATOR +
        escapeHtml(String(focused.label)) +
        "</span>"
      : "";
    var title = escapeHtml(String((main && main.label) || "PATCH"));
    return workspaceScaffold(
      model,
      '<span class="type-label muted">' + title + "</span>",
      annotation,
      body
    );
  }

  // ---- the persistent side region -----------------------------------------

  // Dispatches on the document's own side-surface summary — the Inspector
  // reading for a mixerInspector summary, the Utility reading for a
  // patchUtility one. A missing side surface says so; an incoherent summary
  // is marked, never guessed at.
  function sideRegionHtml(model) {
    var side = persistentSideSurface(model);
    if (!side) {
      // The persistent side region with no surface behind it says so; it is
      // never painted with representative content.
      return (
        '<span class="type-label muted">CURSOR</span>' +
        '<span class="type-value muted" data-role="cursor">' +
        UNAVAILABLE +
        "</span>"
      );
    }
    var summary = side.summary || {};
    if (summary.kind === "mixerInspector") {
      return mixerInspectorHtml(model, side);
    }
    if (summary.kind === "patchUtility") {
      return patchUtilityHtml(model, side, summary);
    }
    // A main-surface summary in the side region is incoherent: the panel has
    // no reading for it and will not invent one.
    return (
      '<span class="type-label muted">PANEL SUMMARY</span>' +
      '<span class="type-value muted">' +
      UNAVAILABLE_MARK +
      "</span>"
    );
  }

  // The PATCH Utility panel: the surface title, the projected patch identity
  // caption, then the five designed entries in authored order — each backed
  // by its projected driver row or marked explicitly unavailable — then any
  // remaining projected entries the designed set does not name
  // (utility_inspector_panel::render_patch_utility).
  function patchUtilityHtml(model, surface, summary) {
    var mode = model.interactionMode;
    var controls = surface.controls || [];
    var byId = {};
    for (var i = 0; i < controls.length; i += 1) {
      var control = controls[i];
      var id = String(
        control.path && control.path.controlId ? control.path.controlId.id : ""
      );
      byId[id] = control;
    }
    var rows = "";
    var claimed = {};
    for (var d = 0; d < DESIGNED_UTILITY_ENTRIES.length; d += 1) {
      var entry = DESIGNED_UTILITY_ENTRIES[d];
      var driven =
        entry.driver !== null &&
        Object.prototype.hasOwnProperty.call(byId, entry.driver) &&
        byId[entry.driver].visible
          ? byId[entry.driver]
          : null;
      if (driven) {
        claimed[entry.driver] = true;
        rows += patchRowHtml(driven, mode, "panel");
      } else {
        rows += markUnavailableRowHtml(entry.label);
      }
    }
    for (var r = 0; r < controls.length; r += 1) {
      var remaining = controls[r];
      var remainingId = String(
        remaining.path && remaining.path.controlId
          ? remaining.path.controlId.id
          : ""
      );
      if (claimed[remainingId] || !remaining.visible) {
        continue;
      }
      rows += patchRowHtml(remaining, mode, "panel");
    }
    var identity =
      '<span class="type-hint focus" data-role="patch-identity">' +
      escapeHtml(String(summary.patch_id)) +
      HINT_SEPARATOR +
      escapeHtml(String(summary.capability_id)) +
      "</span>";
    return (
      '<span class="type-label muted">' +
      escapeHtml(String(surface.label || "UTILITY")) +
      "</span>" +
      identity +
      rows
    );
  }

  // The MIXER Inspector reading (unchanged from the shipped MIXER page).
  function mixerInspectorHtml(model, inspector) {
    var main = surfaceById(model, "mixerMain");

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

  // CSP: the production policy (style-src 'self', no unsafe-inline) blocks
  // every parsed inline style attribute, but CSSOM property assignment is
  // exempt by spec — dynamic geometry must go through here. The rendered
  // HTML carries the value as a data attribute (data-level / data-position);
  // this one pass reads those attributes back and applies the custom
  // properties page.css consumes. A pure function of the DOM — no state, no
  // memory of prior renders — so the same document still paints the same
  // geometry (determinism proof target). Absence of the attribute means "no
  // data", never "zero": unavailable branches emit no attribute and get no
  // property.
  function applyDynamicGeometry(doc) {
    var levels = doc.querySelectorAll("[data-level]");
    for (var i = 0; i < levels.length; i += 1) {
      levels[i].style.setProperty(
        "--level",
        levels[i].getAttribute("data-level")
      );
    }
    var positions = doc.querySelectorAll("[data-position]");
    for (var j = 0; j < positions.length; j += 1) {
      positions[j].style.setProperty(
        "--position",
        positions[j].getAttribute("data-position")
      );
    }
  }

  // Rebuilds the five shell bands from one deserialized
  // SemanticGraphicalViewModel document. Same document, identical DOM. The
  // context line, identity header, workspace scaffold, side region, and
  // footer are the same shared bands for both contexts; only the workspace
  // body and the side reading follow the document's own context.
  function render(model) {
    var doc = window.document;
    var main = surfaceById(model, "mixerMain");
    var columns = trackColumns(main);
    doc.getElementById("context-line").innerHTML = contextLineHtml(model);
    doc.getElementById("identity-header").innerHTML = identityHeaderHtml(
      model,
      columns
    );
    doc.getElementById("workspace").innerHTML =
      model.context === "patch"
        ? patchWorkspaceHtml(model)
        : mixerWorkspaceHtml(model, columns);
    doc.getElementById("inspector").innerHTML = sideRegionHtml(model);
    doc.getElementById("footer").innerHTML = footerHtml(model);
    // Final step, after ALL five region insertions: apply the dynamic
    // geometry in the same paint, on the initial render and every re-render
    // alike. render() is the single place projection content enters the DOM.
    applyDynamicGeometry(doc);
  }

  // ---- the structural observation (acceptance harness contract) -----------

  // Renders the document, then reads the painted DOM back into post-paint
  // structural evidence. Everything in the returned object is copied from
  // what was actually painted — not from the input document — so a plan that
  // never reached the screen cannot satisfy it. The MIXER fields (columns,
  // inspector, meter) and the PATCH fields (rows, utility, annotations) are
  // both always present; whichever context did not paint reports its empty
  // shape, deterministically.
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

    // The painted PATCH strip rows, in painted order, with the state each
    // row was painted in — the PATCH twin of the mixer column report.
    function rowReport(nodes) {
      var out = [];
      for (var r = 0; r < nodes.length; r += 1) {
        var rowNode = nodes[r];
        out.push({
          control: rowNode.getAttribute("data-control"),
          state: rowNode.getAttribute("data-state"),
          label: textOf(rowNode, ".prow-label"),
          value: textOf(rowNode, ".prow-value"),
          mark: textOf(rowNode, '[data-role="state-mark"]'),
        });
      }
      return out;
    }
    var rows = rowReport(doc.querySelectorAll("#strip .prow"));
    var lifecycles = [];
    var lifecycleNodes = doc.querySelectorAll("#workspace .lifecycle");
    for (var l = 0; l < lifecycleNodes.length; l += 1) {
      lifecycles.push({
        control: lifecycleNodes[l].getAttribute("data-lifecycle-for"),
        text: lifecycleNodes[l].textContent.replace(/\s+/g, " ").trim(),
      });
    }

    var inspectorElement = doc.getElementById("inspector");
    var sends = [];
    var sendRows = inspectorElement.querySelectorAll(
      '[data-role="sends"] tr[data-send]'
    );
    for (var r2 = 0; r2 < sendRows.length; r2 += 1) {
      var cells = sendRows[r2].querySelectorAll("td");
      sends.push({
        label: cells[0].textContent.trim(),
        value: cells[1].textContent.trim(),
      });
    }
    var utility = rowReport(inspectorElement.querySelectorAll(".prow"));

    var focusedNode = doc.querySelector("#bank .column.focused");
    return {
      generation: model.generation,
      stateHash: model.stateHash,
      bands: bands,
      columns: columns,
      rows: rows,
      lifecycles: lifecycles,
      sectionAnnotation: textOf(doc, '[data-role="section-annotation"]'),
      patchIdentity: textOf(doc, '[data-role="patch-identity"]'),
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
        utility: utility,
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
  // revision), the zero state otherwise — the retired-shell stale rule, verbatim.
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

  // The paint acknowledgment for one painted document: the document's
  // semantic identity — generation, stateHash, context, active surface,
  // focus path, interaction mode — copied verbatim from the received
  // document (never invented, cached, or re-derived), plus the post-paint
  // measured viewport and per-region bounds the adapter's
  // ShellFrameObservation forwarding consumes (WP02). The region ids spell
  // ShellRegionId's serialized names so the Rust side can assemble honest
  // observations without re-deriving them.
  function paintedEvidence(model) {
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
      generation: model.generation,
      stateHash: model.stateHash,
      context: model.context,
      activeSurface: model.activeSurface,
      focusPath: model.focusPath,
      interactionMode: model.interactionMode,
      viewport: {
        widthPx: window.innerWidth,
        heightPx: window.innerHeight,
      },
      regions: regions,
    };
  }

  // The page half of the typed render-failure path: one small
  // JSON-serializable payload — error name, message, and the failing
  // document's semantic identity (the same generation/stateHash pair the
  // painted ack leads with) when a document is available — emitted on
  // crest://render-error, which the shell converts to the fatal typed
  // PageRenderFailed error. First error wins: the shell treats the first
  // payload as fatal, so the latch drops every later fault instead of
  // letting it replace the recorded failure. The boundary itself does no
  // rendering, no DOM access, and no formatting beyond these strings, so it
  // cannot throw before emitting.
  var renderErrorEmitted = false;

  function emitRenderError(error, model) {
    if (renderErrorEmitted) {
      return; // first error wins; the shell treats the first payload as fatal
    }
    renderErrorEmitted = true;
    var tauri = window.__TAURI__;
    if (!tauri || !tauri.event) {
      return; // headless harness: the throw already propagated to the caller
    }
    tauri.event.emit(RENDER_ERROR_EVENT, {
      name: error && error.name ? String(error.name) : "Error",
      message: error && error.message ? String(error.message) : String(error),
      generation:
        model && model.generation !== undefined ? model.generation : null,
      stateHash:
        model && model.stateHash !== undefined ? model.stateHash : null,
    });
  }

  function attachTransports() {
    var tauri = window.__TAURI__;
    if (!tauri || !tauri.event) {
      return; // headless harness drives window.crest.render directly
    }
    tauri.event.listen(PROJECTION_EVENT, function (event) {
      var model = event.payload;
      latestModel = model;
      // A document that fails to render must NOT ack: the catch emits the
      // typed crest://render-error payload (fatal shell-side) and returns
      // before the acknowledgment below is ever scheduled.
      try {
        render(model);
        updateMeter();
      } catch (error) {
        emitRenderError(error, model);
        return; // a failed render must NOT ack
      }
      // Exactly one paint ack per painted document, in paint order: emitted
      // after this frame has painted, carrying the document's own identity
      // with post-paint region evidence.
      window.requestAnimationFrame(function () {
        tauri.event.emit(PAINTED_EVENT, paintedEvidence(model));
      });
    });
    tauri.event.listen(METER_EVENT, function (event) {
      latestFrame = event.payload;
      updateMeter();
    });
  }

  // Any uncaught page fault after load — including a throw inside the
  // requestAnimationFrame ack callback (paintedEvidence) — is the same typed
  // fatal condition. Both handlers share the emitRenderError first-error
  // latch with the projection listener's boundary. These are fault
  // reporters, not input capture: the page still registers no key handler.
  window.addEventListener("error", function (event) {
    emitRenderError(
      event.error || { name: "Error", message: event.message },
      latestModel
    );
  });
  window.addEventListener("unhandledrejection", function (event) {
    emitRenderError(
      event.reason instanceof Error
        ? event.reason
        : { name: "UnhandledRejection", message: String(event.reason) },
      latestModel
    );
  });

  window.crest = { render: render, renderObservation: renderObservation };
  attachTransports();
})();
