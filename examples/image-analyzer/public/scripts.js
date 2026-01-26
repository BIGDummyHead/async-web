const uploadArea = document.getElementById("uploadArea");
const fileInput = document.getElementById("fileInput");
const preview = document.getElementById("preview");
const uploadBtn = document.getElementById("uploadBtn");
const output = document.getElementById("output");
const status = document.getElementById("status");

let selectedFile = null;

// Click to select file
uploadArea.addEventListener("click", () => fileInput.click());

// File input change
fileInput.addEventListener("change", (e) => {
  const file = e.target.files[0];
  if (file) handleFile(file);
});

// Drag and drop
uploadArea.addEventListener("dragover", (e) => {
  e.preventDefault();
  uploadArea.classList.add("dragover");
});

uploadArea.addEventListener("dragleave", () => {
  uploadArea.classList.remove("dragover");
});

uploadArea.addEventListener("drop", (e) => {
  e.preventDefault();
  uploadArea.classList.remove("dragover");
  const file = e.dataTransfer.files[0];
  if (file && file.type.startsWith("image/")) {
    handleFile(file);
  }
});

function handleFile(file) {
  selectedFile = file;

  // Show preview
  const reader = new FileReader();
  reader.onload = (e) => {
    preview.innerHTML = `<img src="${e.target.result}" alt="Preview">`;
    uploadBtn.disabled = false;
  };
  reader.readAsDataURL(file);
}

uploadBtn.addEventListener("click", async () => {
  if (!selectedFile) return;

  output.textContent = "";
  uploadBtn.disabled = true;
  showStatus("loading", "Generating alt text...");

  try {
    const response = await fetch("/alt", {
      method: "POST",
      headers: {
        "Content-Type": selectedFile.type,
      },
      body: selectedFile,
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();

    showStatus("loading", "Streaming response...");

    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        showStatus("success", "Complete!");
        break;
      }

      const chunk = decoder.decode(value, { stream: true });
      output.textContent += chunk;
    }
  } catch (error) {
    console.error("Error:", error);
    showStatus("error", `Error: ${error.message}`);
    output.textContent =
      "Failed to generate alt text. Check console for details.";
  } finally {
    uploadBtn.disabled = false;
  }
});

function showStatus(type, message) {
  status.className = `status ${type}`;
  status.textContent = message;
}
