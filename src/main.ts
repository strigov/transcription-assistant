import './styles/main.css';

// Add loading indicator
console.log('Loading Transcription Assistant...');
document.addEventListener('DOMContentLoaded', () => {
  console.log('DOM loaded, starting app...');
  initApp();
});

async function initApp() {
  try {
    const { invoke } = await import('@tauri-apps/api/tauri');
    const { open } = await import('@tauri-apps/api/dialog');
    const { listen } = await import('@tauri-apps/api/event');
    
    console.log('Tauri APIs loaded successfully');
    const app = new TranscriptionAssistant(invoke, open, listen);
    (window as any).app = app;
  } catch (error) {
    console.error('Failed to load Tauri APIs:', error);
    alert('Error loading application: ' + error);
  }
}

class TranscriptionAssistant {
  private selectedFile: string | null = null;
  private transcriptionFiles: string[] = [];
  private invoke: any;
  private open: any;
  private listen: any;

  constructor(invoke: any, open: any, listen: any) {
    this.invoke = invoke;
    this.open = open;
    this.listen = listen;
    this.initializeEventListeners();
    this.setupTauriEventListeners();
  }

  private initializeEventListeners() {
    const selectFileBtn = document.getElementById('selectFileBtn')!;
    const startProcessingBtn = document.getElementById('startProcessingBtn')!;
    const selectTranscriptionBtn = document.getElementById('selectTranscriptionBtn')!;
    const mergeBtn = document.getElementById('mergeBtn')!;
    const exportBtn = document.getElementById('exportBtn')!;
    const fileDropZone = document.getElementById('fileDropZone')!;

    selectFileBtn.addEventListener('click', this.selectFile.bind(this));
    startProcessingBtn.addEventListener('click', this.startProcessing.bind(this));
    selectTranscriptionBtn.addEventListener('click', this.selectTranscriptionFiles.bind(this));
    mergeBtn.addEventListener('click', this.mergeTranscriptions.bind(this));
    exportBtn.addEventListener('click', this.exportResults.bind(this));

    // Temporarily disable drag & drop until Tauri integration is fixed
    // TODO: Implement proper Tauri file drop events
  }

  private async setupTauriEventListeners() {
    await this.listen('processing-progress', (event: any) => {
      this.updateProgress(event.payload.progress, event.payload.message);
    });

    await this.listen('processing-complete', (event: any) => {
      this.onProcessingComplete(event.payload);
    });
  }

  private async selectFile() {
    try {
      const selected = await this.open({
        multiple: false,
        filters: [{
          name: 'Media Files',
          extensions: ['mp4', 'avi', 'mov', 'mkv', 'webm', 'flv', 'wmv', 'mp3', 'wav', 'aac', 'flac', 'ogg', 'm4a', 'wma', 'opus']
        }]
      });

      if (selected && typeof selected === 'string') {
        this.selectedFile = selected;
        await this.displayFileInfo(selected);
        (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
      }
    } catch (error) {
      console.error('Error selecting file:', error);
    }
  }

  private async displayFileInfo(filePath: string) {
    try {
      const fileInfo = await this.invoke('get_file_info', { path: filePath });
      const fileName = document.getElementById('fileName')!;
      const fileDuration = document.getElementById('fileDuration')!;
      const fileSize = document.getElementById('fileSize')!;
      const fileInfoDiv = document.getElementById('fileInfo')!;

      fileName.textContent = (fileInfo as any).name;
      fileDuration.textContent = (fileInfo as any).duration;
      fileSize.textContent = (fileInfo as any).size;
      fileInfoDiv.style.display = 'block';
    } catch (error) {
      console.error('Error getting file info:', error);
    }
  }

  private async startProcessing() {
    if (!this.selectedFile) return;

    const maxDuration = parseInt((document.getElementById('maxDuration') as HTMLInputElement).value);
    const useSilenceDetection = (document.getElementById('silenceDetection') as HTMLInputElement).checked;

    try {
      document.getElementById('progressSection')!.style.display = 'block';
      (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = true;

      await this.invoke('start_audio_processing', {
        filePath: this.selectedFile,
        maxDuration: maxDuration * 60, // Convert to seconds
        useSilenceDetection
      });
    } catch (error) {
      console.error('Error starting processing:', error);
      (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
    }
  }

  private updateProgress(progress: number, message: string) {
    const progressFill = document.getElementById('progressFill')!;
    const progressText = document.getElementById('progressText')!;

    progressFill.style.width = `${progress}%`;
    progressText.textContent = message;
  }

  private onProcessingComplete(result: any) {
    console.log('Processing complete:', result);
    this.updateProgress(100, 'Processing complete!');
    (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
    
    // Show results section and display processed segments
    document.getElementById('resultsSection')!.style.display = 'block';
    this.displayProcessingResults(result);
  }

  private displayProcessingResults(result: any) {
    const resultsDiv = document.getElementById('processingResults')!;
    resultsDiv.innerHTML = '';

    if (result.segments && Array.isArray(result.segments)) {
      const segmentsContainer = document.createElement('div');
      segmentsContainer.className = 'segments-container';
      
      const title = document.createElement('h3');
      title.textContent = `✅ Created ${result.segments.length} audio segments:`;
      segmentsContainer.appendChild(title);

      result.segments.forEach((segment: any, index: number) => {
        const segmentItem = document.createElement('div');
        segmentItem.className = 'segment-item';
        segmentItem.innerHTML = `
          <div class="segment-info">
            <strong>Segment ${index + 1}</strong>
            <span class="segment-duration">${segment.duration || 'Unknown duration'}</span>
            <span class="segment-path">${segment.path || 'Unknown path'}</span>
          </div>
          <div class="segment-actions">
            <button onclick="navigator.clipboard.writeText('${segment.path}')">📋 Copy Path</button>
            <button onclick="app.openFolder('${segment.path}')">📁 Open Folder</button>
          </div>
        `;
        segmentsContainer.appendChild(segmentItem);
      });

      resultsDiv.appendChild(segmentsContainer);
    } else {
      resultsDiv.innerHTML = '<p>❌ No segments were created. Please check the processing logs.</p>';
    }
  }

  private async selectTranscriptionFiles() {
    try {
      const selected = await this.open({
        multiple: true,
        filters: [{
          name: 'Text Files',
          extensions: ['txt', 'srt', 'md']
        }]
      });

      if (selected && Array.isArray(selected)) {
        this.transcriptionFiles = selected;
        this.displayTranscriptionFiles();
        (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = false;
      }
    } catch (error) {
      console.error('Error selecting transcription files:', error);
    }
  }

  private displayTranscriptionFiles() {
    const listElement = document.getElementById('transcriptionList')!;
    listElement.innerHTML = '';

    this.transcriptionFiles.forEach((file, index) => {
      const fileItem = document.createElement('div');
      fileItem.className = 'transcription-item';
      fileItem.innerHTML = `
        <span>${file.split('/').pop()}</span>
        <button onclick="app.removeTranscriptionFile(${index})">Remove</button>
      `;
      listElement.appendChild(fileItem);
    });
  }

  public removeTranscriptionFile(index: number) {
    this.transcriptionFiles.splice(index, 1);
    this.displayTranscriptionFiles();
    (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = this.transcriptionFiles.length === 0;
  }

  public async openFolder(filePath: string) {
    try {
      // Get the directory containing the file
      const directory = filePath.substring(0, filePath.lastIndexOf('/'));
      await this.invoke('open_folder', { path: directory });
    } catch (error) {
      console.error('Error opening folder:', error);
      // Fallback: copy path to clipboard
      navigator.clipboard.writeText(filePath);
      alert('Could not open folder. File path copied to clipboard.');
    }
  }

  private async mergeTranscriptions() {
    if (this.transcriptionFiles.length === 0) return;

    const mergeBtn = document.getElementById('mergeBtn') as HTMLButtonElement;
    const originalText = mergeBtn.textContent;
    
    try {
      mergeBtn.disabled = true;
      mergeBtn.textContent = '🔄 Merging...';
      
      const outputFormat = (document.getElementById('outputFormat') as HTMLSelectElement).value;
      
      const result = await this.invoke('merge_transcriptions', {
        files: this.transcriptionFiles,
        outputFormat
      });

      console.log('Merge complete:', result);
      mergeBtn.textContent = '✅ Merged!';
      (document.getElementById('exportBtn') as HTMLButtonElement).disabled = false;
      
      // Show success message
      this.showMergeStatus('✅ Transcriptions merged successfully! Ready for export.', 'success');
      
      setTimeout(() => {
        mergeBtn.textContent = originalText;
        mergeBtn.disabled = false;
      }, 2000);
      
    } catch (error) {
      console.error('Error merging transcriptions:', error);
      mergeBtn.textContent = originalText;
      mergeBtn.disabled = false;
      
      const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
      this.showMergeStatus(`❌ Error merging transcriptions: ${errorMessage}`, 'error');
    }
  }

  private showMergeStatus(message: string, type: 'success' | 'error') {
    const statusDiv = document.createElement('div');
    statusDiv.className = `merge-status ${type}`;
    statusDiv.textContent = message;
    
    const mergeSection = document.querySelector('.merge-section');
    const existingStatus = mergeSection?.querySelector('.merge-status');
    if (existingStatus) {
      existingStatus.remove();
    }
    
    mergeSection?.appendChild(statusDiv);
    
    setTimeout(() => {
      statusDiv.remove();
    }, 5000);
  }

  private async exportResults() {
    const exportBtn = document.getElementById('exportBtn') as HTMLButtonElement;
    const originalText = exportBtn.textContent;
    
    try {
      exportBtn.disabled = true;
      exportBtn.textContent = '📤 Exporting...';
      
      const result = await this.invoke('export_merged_transcription');
      console.log('Export complete:', result);
      
      exportBtn.textContent = '✅ Exported!';
      
      // Show success message
      const message = result?.path 
        ? `✅ File exported successfully to: ${result.path}`
        : '✅ Export completed successfully!';
      this.showExportStatus(message, 'success');
      
      setTimeout(() => {
        exportBtn.textContent = originalText;
        exportBtn.disabled = false;
      }, 2000);
      
    } catch (error) {
      console.error('Error exporting:', error);
      exportBtn.textContent = originalText;
      exportBtn.disabled = false;
      
      const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
      this.showExportStatus(`❌ Error exporting file: ${errorMessage}`, 'error');
    }
  }

  private showExportStatus(message: string, type: 'success' | 'error') {
    const statusDiv = document.createElement('div');
    statusDiv.className = `export-status ${type}`;
    statusDiv.textContent = message;
    
    const outputSection = document.querySelector('.output-section');
    const existingStatus = outputSection?.querySelector('.export-status');
    if (existingStatus) {
      existingStatus.remove();
    }
    
    outputSection?.appendChild(statusDiv);
    
    setTimeout(() => {
      statusDiv.remove();
    }, 7000);
  }

  private handleDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer!.dropEffect = 'copy';
    (e.target as HTMLElement).style.backgroundColor = '#f0f4ff';
  }

  private handleDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    (e.target as HTMLElement).style.backgroundColor = '';
  }

  private async handleFileDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    (e.target as HTMLElement).style.backgroundColor = '';
    
    const files = e.dataTransfer!.files;
    if (files.length > 0) {
      const file = files[0];
      console.log('Dropped file:', file.name);
      
      try {
        // In Tauri, dropped files should provide file paths through file.path
        const filePath = (file as any).path;
        if (filePath) {
          this.selectedFile = filePath;
          await this.displayFileInfo(filePath);
          (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
        } else {
          console.error('Could not get file path from dropped file');
          alert('Error: Could not get file path. Please try using the "Select File" button instead.');
        }
      } catch (error) {
        console.error('Error handling dropped file:', error);
        alert('Error handling dropped file: ' + error);
      }
    }
  }

  private async handleTranscriptionDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    (e.target as HTMLElement).style.backgroundColor = '';
    
    const files = e.dataTransfer!.files;
    const filePaths: string[] = [];
    
    try {
      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        if (file.name.endsWith('.txt') || file.name.endsWith('.srt') || file.name.endsWith('.md')) {
          const filePath = (file as any).path;
          if (filePath) {
            filePaths.push(filePath);
          } else {
            console.warn(`Could not get path for file: ${file.name}`);
          }
        }
      }
      
      if (filePaths.length > 0) {
        this.transcriptionFiles = filePaths;
        this.displayTranscriptionFiles();
        (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = false;
      } else if (files.length > 0) {
        alert('Could not get file paths from dropped files. Please try using the "Select Files" button instead.');
      }
    } catch (error) {
      console.error('Error handling dropped transcription files:', error);
      alert('Error handling dropped files: ' + error);
    }
  }
}