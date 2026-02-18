<script>
  let URL = "";
  let sessionStarted = false;
  let currentPacket;
  let tableHost;
  let fileForm;

  let packetIndex = 0;
  let firstTime = 0;
  let packetCounter = 0;

  let Packets = [];
  let filter = "";
  let JSONfilter = "";

  let scrollToIndex = () => {};
  let scrollToIndexFilter = () => {};
  let endIndex, filterEndIndex;
  import VirtualList from "svelte-virtual-list-ce";
  import Packet from "./Packet.svelte";
  import { tick, onMount } from "svelte";

  import materialDarker from "svelte-highlight/src/styles/material-darker";
  import { JSONEditor } from "svelte-jsoneditor";
  import "svelte-jsoneditor/themes/jse-theme-dark.css";
  import { protoRawDecode } from "./proto_raw_decoder"

  let showSettings = false;
  let startPort = "";
  let endPort = "";
  let packetFilterList = [];
  let newProtocol = "";
  let editingIndex = -1;
  let editingValue = "";
  let autoSavePcapFiles = true;
  let useTcp = false;

  const stream = new EventSource(URL + "/api/stream");
  stream.addEventListener("open", (e) => {
    console.log("SSE connected" + JSON.stringify(e));
  });

  stream.addEventListener("packetNotify", (e) => {
    const { time, fromServer, packetId, packetName, object, raw } = JSON.parse(
      e.data
    );
    
    packetCounter++;

    console.log(time, fromServer, packetId, packetName, object, raw);
    if (packetId === 0) {
      packetIndex = 0;
      firstTime = time;
    }
    packetIndex++;

    const newPacket = {
      globalId: packetCounter,
      reltime: time - firstTime === 0 ? null : (time - firstTime) / 1000,
      index: packetIndex,
      packetID: packetId,
      protoName: packetName,
      source: !fromServer,
      object: object,
      packet: raw,
      decode: protoRawDecode(raw)
    };
	 
    tick().then(() => {
      Packets = Packets.concat(newPacket);
    });
  });

  function startSession() {
    fetch(URL + "/api/start");
    sessionStarted = true;
  }
  function stopSession() {
    fetch(URL + "/api/stop");
    sessionStarted = false;
  }

  function uploadFile() {
    fileForm.click();
  }
  function sendFile(e) {
    const formData = new FormData();
    formData.append("file", this.files[0]);
    fetch(URL + "/api/upload", {
      method: "POST",
      body: formData,
    });
    fileForm.value = "";
  }

  let editor,
    decodeEditor = true;
  let editorCss = "";
  function showPacketDetails(packet) {
    currentPacket = packet;
    tick().then(() => {
      if (packet.object && packet.decode && showDecode) {
        editorCss = "two-editor";
      } else {
        editorCss = "one-editor";
      }
      if (!packet.object && packet.decode) {
        showDecode = true;
      }
      if (packet.object) {
        editor.set({ json: packet.object });
      }
      if (showDecode && packet.decode) {
        decodeEditor.set({ json: packet.decode });
      }
    });
  }

  let showDecode = false;
  function handleShowDecode() {
    tick().then(() => {
      showDecode = !showDecode;
      showPacketDetails(currentPacket);
    });
  }
  function handleRenderMenu(mode, items) {
    const separator = {
      separator: true,
    };
    const rawDecButton = {
      onClick: handleShowDecode,
      text: "RD",
      title: "Raw Decode",
      className: "jse-button raw-decode-btn",
    };

    const space = {
      space: true,
    };
    const itemsWithoutSpace = items.slice(0, items.length - 1);
    return itemsWithoutSpace.concat([separator, rawDecButton, space]);
  }

  function scrollToEnd() {
    if (Packets.length > 1) {
      scrollToIndex(Packets.length - 1, { behavior: "auto" });
    }
  }

  function downloadAll() {
    const saveTemplateAsFile = (filename, dataObjToWrite) => {
      const blob = new Blob([JSON.stringify(dataObjToWrite)], {
        type: "text/json",
      });
      const link = document.createElement("a");

      link.download = filename;
      link.href = window.URL.createObjectURL(blob);
      link.dataset.downloadurl = ["text/json", link.download, link.href].join(
        ":"
      );

      const evt = new MouseEvent("click", {
        view: window,
        bubbles: true,
        cancelable: true,
      });

      link.dispatchEvent(evt);
      link.remove();
    };
    saveTemplateAsFile("capture.json", FilteredPackets);
  }
  function copyToClipboard(text) {
    if (window.clipboardData && window.clipboardData.setData) {
      return window.clipboardData.setData("Text", text);
    } else if (
      document.queryCommandSupported &&
      document.queryCommandSupported("copy")
    ) {
      var textarea = document.createElement("textarea");
      textarea.textContent = text;
      textarea.style.position = "fixed";
      document.body.appendChild(textarea);
      textarea.select();
      try {
        return document.execCommand("copy");
      } catch (ex) {
        console.warn("Copy to clipboard failed.", ex);
        return prompt("Copy to clipboard: Ctrl+C, Enter", text);
      } finally {
        document.body.removeChild(textarea);
      }
    }
  }
  function copyCurrentPacket() {
    copyToClipboard(JSON.stringify(currentPacket));
  }
  function copyCurrentBin() {
    copyToClipboard(currentPacket.packet);
  }

  function clear() {
    Packets = [];
    packetCounter = 0;
    tick().then(() => {
      scrollToIndex(0);
    });
  }

  function resizeHandler(e) {
    const rect = node.getBoundingClientRect();
    function stopResize(e) {
      document.removeEventListener("mouseup", stopResize);
      document.removeEventListener("mousemove", move);
      node.style.userSelect = null;
    }
    function move(e) {
      details.style.width =
        100 - ((e.clientX - rect.left) / node.offsetWidth) * 100 + "%";
    }
    node.style.userSelect = "none";
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", stopResize);
  }

  let stick;

  function packetFilter(packet) {
    if (!filter.length && !JSONfilter.length) return false;
    let and = filter.length && JSONfilter.length;
    if (!orand) and = false;
    let text, json;
    if (
      filter.length &&
      packet.protoName.includes &&
      packet.protoName.toLowerCase().includes(filter.toLowerCase())
    )
      text = true;
    if (filter.length && ("" + packet.packetID).includes(filter)) text = true;
    if (!and && text) return true;
    if (
      JSONfilter.length &&
      packet.object &&
      JSON.stringify(packet.object)
        .toLowerCase()
        .includes(JSONfilter.toLowerCase())
    )
      json = true;
    if (!and && json) return true;
    if (and && text && json) return true;
    return false;
  }
  let FilteredPackets = [];
  $: if (filter.length || JSONfilter.length || orand) {
    tick().then(() => {
      FilteredPackets = Packets.filter(packetFilter, filter, JSONfilter);
      setTimeout(() => {
        scrollToIndexFilter(10, { behavior: "auto" });
        scrollToIndexFilter(0, { behavior: "auto" });
      }, 10);
    });
  } else {
    tick().then(() => {
      FilteredPackets = [];
    });
  }
  $: {
    tick().then(() => scrollToEnd(filter, JSONfilter));
  }
  $: {
    if (stick) tick().then(() => scrollToEnd(Packets));
  }

  onMount(async () => {
    fileForm.addEventListener("change", sendFile, false);
    
    const response = await fetch(`${URL}/api/GetConfig`);
    if (response.ok) {
      const data = await response.json();
      startPort = data.minKcpPort.toString();
      endPort = data.maxKcpPort.toString();
      packetFilterList = (data.packetFilter || [])
        .filter(p => p.trim() !== "");
      autoSavePcapFiles = data.autoSavePcapFiles;
      useTcp = data.useTcp || false;
    }
  });

  let node,
    details,
    filterTableHost,
    orand = true;

  function addProtocol() {
    const protocol = newProtocol.trim();
    if (protocol) {
      if (packetFilterList.includes(protocol)) {
        newProtocol = "";
        return;
      }
      packetFilterList = [...packetFilterList, protocol];
      newProtocol = "";
    }
  }

  function startEdit(index) {
    editingIndex = index;
    editingValue = packetFilterList[index];
  }

  function saveEdit() {
    if (editingIndex >= 0) {
      const value = editingValue.trim();
      if (value) {
        packetFilterList = packetFilterList.map((p, i) => 
          i === editingIndex ? value : p
        );
      } else {
        packetFilterList = packetFilterList.filter((_, i) => i !== editingIndex);
      }
      editingIndex = -1;
      editingValue = "";
    }
  }

  function cancelEdit() {
    editingIndex = -1;
    editingValue = "";
  }

  function removeProtocol(index) {
    packetFilterList = packetFilterList.filter((_, i) => i !== index);
    if (editingIndex === index) {
      editingIndex = -1;
      editingValue = "";
    }
  }

  async function saveAllSettings() {
    if (!startPort || !endPort) {
      alert("Please fill in the complete port range");
      return;
    }
    if (parseInt(startPort) > parseInt(endPort)) {
      alert("The start port cannot be larger than the end port");
      return;
    }

    const filteredPacketFilter = packetFilterList.filter(p => p.trim() !== "");
    const settings = {
      minKcpPort: parseInt(startPort),
      maxKcpPort: parseInt(endPort),
      packetFilter: filteredPacketFilter.length > 0 ? filteredPacketFilter : [""],
      autoSavePcapFiles: autoSavePcapFiles,
      useTcp: useTcp
    };

    const response = await fetch(`${URL}/api/SetConfig`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(settings)
    });
      
    if (response.ok) {
      showSettings = false;
    } else {
      alert("Failed to save Settings");
    }
  }
</script>

<svelte:head>
  {@html materialDarker}
</svelte:head>

<aside>
  {#if sessionStarted}
    <button
      title="Stop Capture"
      data-icon="network-off-outline"
      on:click={stopSession}
      class="red"
    />
  {:else}
    <button
      title="Start Capture"
      data-icon="play-network-outline"
      on:click={startSession}
      class="green"
    />
  {/if}
  <button title="Upload PCAP" data-icon="open-in-app" on:click={uploadFile} />
  <input hidden type="file" bind:this={fileForm} accept=".json" />
  <button title="Clear" data-icon="clear" class="red" on:click={clear} />
  <button 
    title="Settings" 
    data-icon="settings" 
    on:click={() => showSettings = true}
  />
  <button
    title="Lock scroll at the bottom"
    data-icon="keyboard_arrow_down"
    style="margin-top: auto;"
    class:green={stick}
    on:click={() => (stick = !stick)}
  />
  {#if currentPacket}
    <button
      title="Copy current bin"
      data-icon="insert_drive_file"
      style="margin-top: auto;"
      on:click={copyCurrentBin}
    />
    <button
      title="Copy current packet"
      data-icon="collections_bookmark"
      on:click={copyCurrentPacket}
    />
  {:else}
    <button
      title="Copy current bin"
      data-icon="insert_drive_file"
      style="margin-top: auto; opacity: 0.5"
    />
    <button
      title="Copy current packet"
      data-icon="collections_bookmark"
      style="opacity: 0.5"
    />
  {/if}
  {#if FilteredPackets && FilteredPackets.length > 0}
    <button
      title="Export all filtered"
      data-icon="download"
      on:click={downloadAll}
    />
  {:else}
    <button
      title="Export all filtered"
      data-icon="download"
      style="opacity: 0.5"
    />
  {/if}
</aside>

{#if showSettings}
  <div class="settings-modal" on:click={() => showSettings = false}>
    <div class="settings-dialog compact" on:click|stopPropagation>
      <h2>Capture Settings</h2>
      
      <div class="setting-row">
        <label>Port Range:</label>
        <div class="port-inputs">
          <input 
            type="number" 
            min="1" 
            max="65535" 
            bind:value={startPort} 
            placeholder="Start"
            class="small-input"
          />
          <span class="dash">–</span>
          <input 
            type="number" 
            min="1" 
            max="65535" 
            bind:value={endPort} 
            placeholder="End"
            class="small-input"
          />
        </div>
      </div>

      <div class="setting-row">
        <label>Use Tcp:</label>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={useTcp} />
          {useTcp ? ' On' : ' Off'}
        </label>
      </div>

      <div class="setting-row">
        <label>Auto Save:</label>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={autoSavePcapFiles} />
          {autoSavePcapFiles ? ' On' : ' Off'}
        </label>
      </div>

      <!-- Packet Filter List -->
      <div class="setting-group compact">
        <h3>Packet Filter</h3>
        <div class="protocol-table compact">
          {#each packetFilterList as protocol, index}
            <div class="filter-item">
              {#if editingIndex === index}
                <input 
                  type="text" 
                  bind:value={editingValue}
                  on:keydown={(e) => {
                    if (e.key === 'Enter') saveEdit();
                    if (e.key === 'Escape') cancelEdit();
                  }}
                  autofocus
                  class="filter-input"
                />
                <div class="filter-actions">
                  <button on:click={saveEdit} title="Save" class="btn-icon">✓</button>
                  <button on:click={cancelEdit} title="Cancel" class="btn-icon">✗</button>
                </div>
              {:else}
                <span class="filter-text" on:dblclick={() => startEdit(index)}>{protocol}</span>
                <div class="filter-actions">
                  <button on:click={() => startEdit(index)} title="Edit" class="btn-icon">✎</button>
                  <button on:click={() => removeProtocol(index)} title="Remove" class="btn-icon">×</button>
                </div>
              {/if}
            </div>
          {/each}
          
          <!-- Add New -->
          <div class="filter-item add">
            <input 
              type="text" 
              bind:value={newProtocol}
              placeholder="Enter new protocol name"
              on:keydown={(e) => e.key === 'Enter' && addProtocol()}
              class="filter-input"
            />
            <button on:click={addProtocol} title="Add" class="btn-icon add-btn">+</button>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="settings-actions compact">
        <button class="btn secondary" on:click|preventDefault={() => showSettings = false}>
          Cancel
        </button>
        <button class="btn primary" on:click|preventDefault={saveAllSettings}>
          Save
        </button>
      </div>
    </div>
  </div>
{/if}

<main bind:this={node}>
  <div class="main-host">
    <div class="filter-host">
      <input type="text" bind:value={filter} placeholder=" PACKET" />
      <div class="orand" on:click={() => (orand = !orand)}>
        <span and class:s={orand}>AND</span>
        <span or class:s={!orand}>OR</span>
      </div>
      <input type="text" bind:value={JSONfilter} placeholder=" JSON" />
    </div>
    <div class="results-host" class:open={filter.length || JSONfilter.length}>
      <div class="table">
        <div class="tr thead">
          <div class="time">Time</div>
          <div class="idx">#</div>
          <div class="src">Sender</div>
          <div class="id">ID</div>
          <div class="name">Proto Name</div>
          <div class="len">Length</div>
          <div class="json">JSON</div>
        </div>
        <div class="tbody" bind:this={filterTableHost}>
          <VirtualList
            items={FilteredPackets}
            let:item={packet}
            bind:scrollToIndexFilter
            bind:end={filterEndIndex}
          >
            <Packet
              {packet}
              idx={packet.index}
              current={packet == currentPacket}
              on:click={() => {
                showPacketDetails(packet);
                const indexInPackets = Packets.findIndex(p => p.globalId === packet.globalId);
                scrollToIndex(Math.max(indexInPackets - 5, 0));
              }}
            />
          </VirtualList>
        </div>
      </div>
    </div>
    <div class="table-host">
      <div class="table">
        <div class="tr thead">
          <div class="time">Time</div>
          <div class="idx">#</div>
          <div class="src">Sender</div>
          <div class="id">ID</div>
          <div class="name">Proto Name</div>
          <div class="len">Length</div>
          <div class="json">JSON</div>
        </div>
        <div class="tbody" bind:this={tableHost}>
          <VirtualList
            items={Packets}
            let:item={packet}
            bind:scrollToIndex
            bind:end={endIndex}
          >
            <Packet
              {packet}
              idx={packet.index}
              current={packet === currentPacket}
              on:click={() => showPacketDetails(packet)}
            />
          </VirtualList>
        </div>
      </div>
    </div>
  </div>
  <div class="resize" on:mousedown={resizeHandler} />
  <div class="details-host" bind:this={details}>
    {#if currentPacket}
      {#if currentPacket.object}
        <div class="{editorCss} jse-theme-dark">
          <JSONEditor
            bind:this={editor}
            onRenderMenu={handleRenderMenu}
            readOnly
          />
        </div>
      {/if}
      {#if currentPacket.decode && showDecode}
        <div class="{editorCss} jse-theme-dark">
          <JSONEditor bind:this={decodeEditor} readOnly />
        </div>
      {/if}
    {/if}
  </div>
</main>

<style>
  .resize {
    /*position: absolute;*/
    top: 0;
    height: 100%;
    width: 10px;
    margin-left: -2px;
    margin-right: -8px;
    background: rgba(255, 255, 255, 0.05);
    z-index: 2;
  }
  .resize:hover {
    background: rgba(255, 255, 255, 0.3);
    cursor: w-resize;
  }

  aside {
    background: rgba(0, 0, 10, 0.4);
    flex-grow: 0;
    display: flex;
    flex-direction: column;
  }

  aside button {
    font-size: 1.75em;
    margin-bottom: 0.3em;
  }

  main {
    display: flex;
    flex-grow: 1;
    overflow: hidden;
  }

  input::placeholder {
    font-family: "shicon", Open Sans, sans-serif;
    color: white;
    letter-spacing: 2px;
    font-size: 0.8em;
  }
  input[type="text"] {
    background: black;
    color: white;
    height: 3rem;
    border: none;
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
    padding: 0 1rem;
    flex-grow: 1;
    font-size: 1.4em;
    font-family: monospace;
  }
  .main-host {
    display: flex;
    flex-direction: column;
    flex-grow: 1;
  }
  .filter-host {
    width: 100%;
    display: flex;
  }
  .table-host {
    flex-grow: 1;
    overflow: hidden;
    background: rgba(0, 0, 10, 0.6);
  }
  .table {
    width: 100%;
    min-width: 100%;
    cursor: default;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }
  .table :global(.tr) {
    display: flex;
  }

  .table :global(.tr > .time) {
    flex-basis: 3rem;
    display: flex;
    justify-content: center;
  }
  .table :global(.tr > .idx) {
    flex-basis: 2.5rem;
    display: flex;
    justify-content: center;
  }

  .table :global(.tr > .src) {
    flex-basis: 4rem;
    display: flex;
    justify-content: center;
  }

  .table :global(.tr > .id) {
    flex-basis: 3rem;
    justify-content: center;
  }
  .table :global(.tr > .name) {
    flex-basis: 20rem;
  }
  .table :global(.tr > .len) {
    flex-basis: 3rem;
  }
  .table :global(.tr > .json) {
    flex-grow: 1;
  }

  .table .thead > * {
    text-align: left;
    background: rgba(0, 0, 0, 0.4);
    border-right: 1px solid rgba(255, 255, 255, 0.1);
    font-size: 0.8em;
  }

  .table .tbody {
    flex-grow: 1;
    flex-shrink: 1;
    overflow: auto;
  }

  .table :global(.tr > *) {
    padding: 0.5rem 0.6rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .table :global(.tr:hover > *) {
    background: rgba(100, 130, 255, 0.2) !important;
  }
  .details-host {
    /*flex-basis: 40%;*/
    /*flex-grow: 1;*/
    width: 30%;
    background: rgba(0, 0, 0, 0.2);
    border-top: 2px solid rgba(255, 255, 255, 0.2);
    overflow-y: auto;
    overflow-x: hidden;
    /*display: flex;*/
    /*flex-direction: column;*/
    max-height: 100%;
  }
  .details-host :global(pre) {
    white-space: pre-wrap !important;
    font-family: monospace;
    line-height: 1.5;
    font-weight: 1000;
    font-size: 1.2em !important;
    background: none !important;
    max-width: 100%;
    overflow: hidden !important;
  }

  .results-host {
    flex-grow: 1;
    max-height: 0%;
    background: black;
  }
  .results-host.open {
    max-height: 30%;
    min-height: 30%;
    border-bottom: 2px white solid;
  }
  .results-host :global(.tr) {
    background: rgba(0, 0, 0, 0.3);
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
  }
  .results-host :global(.tr > div) {
    background: transparent !important;
  }
  .orand {
    font-size: 0.8em;
    box-sizing: border-box;
    padding: 2px 4px;
    background: rgba(0, 0, 0, 0.7);
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
    cursor: pointer;
  }
  .orand:hover span {
    background: rgba(255, 255, 255, 0.1);
  }
  .orand span {
    color: #777;
    font-weight: 700;
    padding: 0.3em 0.2em;
    display: block;
    border-radius: 2px;
    min-width: 2.2em;
    text-align: center;
    user-select: none;
  }
  .orand span[and] {
    margin-bottom: 2px;
  }
  .orand span.s {
    background: #1aa1e7;
    color: white;
  }
  .two-editor {
    height: 50%;
  }
  .one-editor {
    height: 100%;
  }
  .raw-decode-btn {
    width: 80px !important;
  }

  .settings-modal {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }
  
  .settings-dialog.compact {
    background: #1e1e1e;
    border-radius: 6px;
    padding: 1rem;
    min-width: 400px;
    max-width: 90%;
    max-height: 80vh;
    overflow: auto;
    border: 1px solid #404040;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    font-size: 0.9em;
  }
  
  .settings-dialog.compact h2 {
    margin: 0 0 1rem 0;
    font-size: 1.2rem;
    color: #e0e0e0;
    text-align: center;
  }
  
  .setting-row {
    display: flex;
    align-items: center;
    margin-bottom: 0.6rem;
    gap: 0.6rem;
  }
  
  .setting-row label:first-child {
    width: 90px;
    color: #ccc;
    font-weight: 500;
  }
  
  .port-inputs {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex: 1;
  }
  
  .small-input {
    padding: 0.3rem 0.4rem;
    background: #2d2d2d;
    border: 1px solid #555;
    color: #f0f0f0;
    border-radius: 3px;
    font-size: 0.9em;
    width: 70px;
  }
  
  .dash {
    color: #aaa;
    font-weight: bold;
  }
  
  /* Compact Protocol Filter */
  .setting-group.compact {
    margin: 0.8rem 0;
  }
  
  .setting-group.compact h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1em;
    color: #ccc;
    font-weight: 600;
  }
  
  .protocol-table.compact {
    background: #252525;
    border-radius: 4px;
    padding: 0.3rem;
    max-height: 140px;
    overflow-y: auto;
  }
  
  .filter-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.3rem;
    padding: 0.2rem 0.3rem;
    border-radius: 3px;
  }
  
  .filter-item:hover {
    background: #2a2a2a;
  }
  
  .filter-item.add {
    background: #202020;
  }
  
  .filter-input {
    flex: 1;
    padding: 0.1rem 0.35rem;
    background: #252526 !important;
    border: 1px solid #666;
    color: #f0f0f0;
    border-radius: 2px;
    font-size: 0.82em;
    height: 26px !important;
    line-height: 1.2;
    outline: none;
    transition: border-color 0.2s, background-color 0.2s;
  }

  .filter-input:focus {
    border-color: #1e88e5;
    background: #404040;
  }

  .filter-input::placeholder {
    color: #888;
    font-style: italic;
  }
  
  .filter-text {
    flex: 1;
    color: #ddd;
    font-size: 0.85em;
    padding: 0.2rem 0;
    cursor: pointer;
  }
  
  .filter-actions {
    display: flex;
    gap: 0.2rem;
  }
  
  .btn-icon {
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    border: none;
    background: #444;
    color: #ccc;
    cursor: pointer;
    font-size: 0.7em;
    padding: 0;
  }
  
  .btn-icon:hover {
    background: #555;
  }
  
  .add-btn {
    background: #1e88e5 !important;
    color: white !important;
  }
  
  /* Compact Action Buttons */
  .settings-actions.compact {
    display: flex;
    justify-content: space-between;
    margin-top: 1rem;
    padding-top: 0.8rem;
    border-top: 1px solid #444;
  }
  
  .btn {
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
    border: none;
    font-size: 0.9em;
  }
  
  .btn.secondary {
    background: #505050;
    color: #e0e0e0;
  }
  
  .btn.primary {
    background: #1e88e5;
    color: white;
  }
  
  .btn:hover {
    opacity: 0.9;
  }
.small-input:focus,
.filter-input:focus,
.btn-icon:focus {
  outline: none;
}
</style>
