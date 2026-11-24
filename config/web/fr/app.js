"use strict";
const form = document.getElementById("prepare-form");
const agent = document.getElementById("agent");
const url = document.getElementById("issue-url");
const output = document.getElementById("prompt-output");
const status = document.getElementById("status");
const copy = document.getElementById("copy");
const prepare = document.getElementById("prepare");
const showStatus = (key) => { status.textContent = status.dataset[key]; };
fetch("/api/agents", { credentials: "same-origin" })
  .then((response) => response.ok ? response.json() : Promise.reject())
  .then((items) => items.forEach((item) => agent.add(new Option(item.label, item.id))))
  .catch(() => showStatus("error"));
form.addEventListener("submit", async (event) => {
  event.preventDefault();
  output.value = "";
  copy.disabled = true;
  prepare.disabled = true;
  showStatus("loading");
  try {
    const response = await fetch("/api/prepare", {
      method: "POST",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url: url.value, agent_id: agent.value || null })
    });
    if (!response.ok) throw new Error();
    const result = await response.json();
    output.value = result.prompt;
    copy.disabled = false;
    showStatus("success");
  } catch {
    showStatus("error");
  } finally {
    prepare.disabled = false;
  }
});
copy.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(output.value);
    showStatus("copied");
  } catch {
    showStatus("copyError");
  }
});
