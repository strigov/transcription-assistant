
// Add loading indicator
console.log('Загрузка Помощника транскрипции...');
document.addEventListener('DOMContentLoaded', () => {
  console.log('DOM загружен, запуск приложения...');
  initApp();
});

async function initApp() {
  try {
    console.log('🚀 Starting app initialization...');
    const { invoke } = await import('@tauri-apps/api/tauri');
    const { open } = await import('@tauri-apps/api/dialog');
    const { listen } = await import('@tauri-apps/api/event');
    
    console.log('✅ API Tauri успешно загружены');
    console.log('🏗️ Creating TranscriptionAssistant instance...');
    const app = new TranscriptionAssistant(invoke, open, listen);
    (window as any).app = app;
    console.log('✅ App initialization complete');
  } catch (error) {
    console.error('❌ Не удалось загрузить API Tauri:', error);
    alert('Ошибка загрузки приложения: ' + error);
  }
}

class TranscriptionAssistant {
  private selectedFile: string | null = null;
  private transcriptionFiles: string[] = [];
  private invoke: any;
  private open: any;
  private listen: any;
  private lastOutputPath: string | null = null;

  constructor(invoke: any, open: any, listen: any) {
    this.invoke = invoke;
    this.open = open;
    this.listen = listen;
    this.initializeEventListeners();
    this.setupTauriEventListeners();
  }

  private initializeEventListeners() {
    console.log('🔧 Initializing event listeners...');
    
    const selectFileBtn = document.getElementById('selectFileBtn')!;
    const startProcessingBtn = document.getElementById('startProcessingBtn')!;
    const selectTranscriptionBtn = document.getElementById('selectTranscriptionBtn')!;
    const clearAllBtn = document.getElementById('clearAllBtn')!;
    const mergeBtn = document.getElementById('mergeBtn')!;
    const exportBtn = document.getElementById('exportBtn')!;
    const selectOutputPathBtn = document.getElementById('selectOutputPathBtn')!;
    const timecodeFormat = document.getElementById('timecodeFormat') as HTMLSelectElement;
    const fileDropZone = document.getElementById('fileDropZone')!;
    const transcriptionDropZone = document.getElementById('transcriptionDropZone')!;

    console.log('🔧 Elements found:');
    console.log('- fileDropZone:', fileDropZone ? 'found' : 'NOT FOUND');
    console.log('- transcriptionDropZone:', transcriptionDropZone ? 'found' : 'NOT FOUND');

    selectFileBtn.addEventListener('click', this.selectFile.bind(this));
    startProcessingBtn.addEventListener('click', this.startProcessing.bind(this));
    selectTranscriptionBtn.addEventListener('click', this.selectTranscriptionFiles.bind(this));
    clearAllBtn.addEventListener('click', this.clearAllTranscriptions.bind(this));
    mergeBtn.addEventListener('click', this.mergeTranscriptions.bind(this));
    exportBtn.addEventListener('click', this.exportResults.bind(this));
    selectOutputPathBtn.addEventListener('click', this.selectOutputPath.bind(this));
    timecodeFormat.addEventListener('change', this.handleTimecodeFormatChange.bind(this));

    // Setup Tauri file drop listeners
    console.log('🔧 Setting up Tauri file drop listeners...');
    this.setupTauriFileDrop();
    
    console.log('✅ Event listeners initialization complete');
  }

  private async setupTauriEventListeners() {
    await this.listen('processing-progress', (event: any) => {
      this.updateProgress(event.payload.progress, event.payload.message);
    });

    await this.listen('processing-complete', (event: any) => {
      this.onProcessingComplete(event.payload);
    });

    await this.listen('ffmpeg-download-progress', (event: any) => {
      this.updateFFmpegDownloadProgress(event.payload.progress, event.payload.message);
    });
  }

  private async selectFile() {
    try {
      const selected = await this.open({
        multiple: false,
        filters: [{
          name: 'Медиа файлы',
          extensions: ['mp4', 'avi', 'mov', 'mkv', 'webm', 'flv', 'wmv', 'mp3', 'wav', 'aac', 'flac', 'ogg', 'm4a', 'wma', 'opus']
        }]
      });

      if (selected && typeof selected === 'string') {
        // Show loading state
        this.showFileLoadingState(true);
        
        this.selectedFile = selected;
        await this.displayFileInfo(selected);
        (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
      }
    } catch (error) {
      console.error('Ошибка выбора файла:', error);
      this.showFileLoadingState(false);
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
      
      // Hide loading state and show success
      this.showFileLoadingState(false);
      this.showFileSuccessState();
      
      // If FFmpeg was already installed, show ready status
      this.checkFFmpegStatus();
    } catch (error) {
      console.error('Ошибка получения информации о файле:', error);
      // Hide loading state and show error
      this.showFileLoadingState(false);
      this.showFileErrorState('Ошибка анализа файла. Попробуйте еще раз.');
    }
  }

  private checkFFmpegStatus() {
    // If no download progress was shown, FFmpeg is already available
    const statusContainer = document.getElementById('ffmpegStatus')!;
    const progressContainer = document.getElementById('ffmpegDownloadProgress')!;
    const readyStatus = document.getElementById('ffmpegReady')!;
    
    if (progressContainer.style.display !== 'block') {
      statusContainer.style.display = 'block';
      readyStatus.style.display = 'flex';
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
      console.error('Ошибка запуска обработки:', error);
      (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
    }
  }

  private updateProgress(progress: number, message: string) {
    const progressFill = document.getElementById('progressFill')!;
    const progressText = document.getElementById('progressText')!;

    progressFill.style.width = `${progress}%`;
    progressText.textContent = message;
  }

  private updateFFmpegDownloadProgress(progress: number, message: string) {
    const statusContainer = document.getElementById('ffmpegStatus')!;
    const progressContainer = document.getElementById('ffmpegDownloadProgress')!;
    const progressFill = document.getElementById('ffmpegProgressFill')!;
    const progressText = document.getElementById('ffmpegProgressText')!;
    const readyStatus = document.getElementById('ffmpegReady')!;

    // Show status container
    statusContainer.style.display = 'block';
    
    if (progress < 100) {
      // Show download progress
      progressContainer.style.display = 'block';
      readyStatus.style.display = 'none';
      
      progressFill.style.width = `${progress}%`;
      progressText.textContent = message;
    } else {
      // Hide progress and show ready status
      setTimeout(() => {
        progressContainer.style.display = 'none';
        readyStatus.style.display = 'flex';
      }, 1000);
    }
  }

  private onProcessingComplete(result: any) {
    console.log('Обработка завершена:', result);
    this.updateProgress(100, 'Обработка завершена!');
    
    // Add green gradient to completed progress bar
    const progressFill = document.getElementById('progressFill')!;
    progressFill.classList.add('completed');
    
    (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
    
    // Показать раздел результатов и отобразить обработанные сегменты
    document.getElementById('resultsSection')!.style.display = 'block';
    this.displayProcessingResults(result);
  }

  private displayProcessingResults(result: any) {
    const resultsDiv = document.getElementById('processingResults')!;
    resultsDiv.innerHTML = '';

    if (result.segments && Array.isArray(result.segments)) {
      const segments = result.segments;
      
      // Output folder section
      const summaryDiv = document.createElement('div');
      summaryDiv.className = 'processing-summary';
      
      // Output folder info
      const outputFolder = segments[0]?.path ? this.getParentFolder(segments[0].path) : '';
      console.log('Frontend: First segment path:', segments[0]?.path);
      console.log('Frontend: Calculated outputFolder:', outputFolder);
      if (outputFolder) {
        const folderDiv = document.createElement('div');
        folderDiv.className = 'output-folder';
        folderDiv.innerHTML = `
          <div class="folder-info">
            <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
            </svg>
            <div class="folder-details">
              <div class="folder-label">Папка с сегментами:</div>
              <div class="folder-path" title="${outputFolder}">${this.truncatePath(outputFolder)}</div>
            </div>
          </div>
          <button class="btn btn-primary" onclick="app.openSegmentsFolder(\`${outputFolder.replace(/\\/g, '\\\\')}\`)">
            <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-1M10 6l5 5-5 5"></path>
            </svg>
            Открыть папку
          </button>
        `;
        summaryDiv.appendChild(folderDiv);
      }
      
      // Segments list
      const segmentsDiv = document.createElement('div');
      segmentsDiv.className = 'segments-list';
      
      const segmentsHeader = document.createElement('h4');
      segmentsHeader.textContent = 'Список сегментов:';
      segmentsDiv.appendChild(segmentsHeader);
      
      segments.forEach((segment: any, index: number) => {
        const segmentItem = document.createElement('div');
        segmentItem.className = 'segment-item-compact';
        
        const filename = segment.path ? segment.path.split(/[\\/]/).pop() : `segment_${index + 1}`;
        
        segmentItem.innerHTML = `
          <div class="segment-number">${index + 1}</div>
          <div class="segment-details">
            <div class="segment-name">${filename}</div>
            <div class="segment-duration">${segment.duration || 'неизвестно'}</div>
          </div>
        `;
        
        segmentsDiv.appendChild(segmentItem);
      });

      resultsDiv.appendChild(summaryDiv);
      resultsDiv.appendChild(segmentsDiv);
    } else {
      resultsDiv.innerHTML = '<div class="status status-error">❌ Сегменты не были созданы. Проверьте логи обработки.</div>';
    }
  }

  private formatDuration(seconds: number): string {
    if (isNaN(seconds) || seconds === 0) return '0:00';
    
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  private getParentFolder(filePath: string): string {
    // Debug the input path
    console.log('getParentFolder input:', filePath);
    console.log('getParentFolder input length:', filePath.length);
    
    // Find the last occurrence of either slash type
    const lastBackslash = filePath.lastIndexOf('\\');
    const lastForwardSlash = filePath.lastIndexOf('/');
    const lastSlashIndex = Math.max(lastBackslash, lastForwardSlash);
    
    if (lastSlashIndex === -1) {
      console.log('No slash found, returning input');
      return filePath;
    }
    
    const result = filePath.substring(0, lastSlashIndex);
    console.log('getParentFolder result:', result);
    return result;
  }

  private truncatePath(path: string, maxLength: number = 50): string {
    if (path.length <= maxLength) return path;
    return '...' + path.slice(-(maxLength - 3));
  }

  private async selectTranscriptionFiles() {
    try {
      const selected = await this.open({
        multiple: true,
        filters: [{
          name: 'Текстовые файлы',
          extensions: ['txt', 'srt', 'md']
        }]
      });

      if (selected && Array.isArray(selected)) {
        // Sort files automatically by numeric sequence
        this.transcriptionFiles = this.sortTranscriptionFiles(selected);
        this.displayTranscriptionFiles();
        this.setDefaultOutputPath();
        (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = false;
      }
    } catch (error) {
      console.error('Ошибка выбора файлов транскрипции:', error);
    }
  }

  private setDefaultOutputPath() {
    if (this.transcriptionFiles.length > 0) {
      const firstFilePath = this.transcriptionFiles[0];
      const directory = firstFilePath.substring(0, firstFilePath.lastIndexOf('/'));
      const outputPathInput = document.getElementById('outputPath') as HTMLInputElement;
      if (!this.lastOutputPath) {
        outputPathInput.value = directory;
      }
    }
  }

  private async selectOutputPath() {
    try {
      const selected = await this.open({
        directory: true,
        multiple: false,
        defaultPath: this.lastOutputPath || undefined
      });

      if (selected && typeof selected === 'string') {
        this.lastOutputPath = selected;
        const outputPathInput = document.getElementById('outputPath') as HTMLInputElement;
        outputPathInput.value = selected;
      }
    } catch (error) {
      console.error('Ошибка выбора папки:', error);
    }
  }

  private handleTimecodeFormatChange() {
    const timecodeFormat = document.getElementById('timecodeFormat') as HTMLSelectElement;
    const customGroup = document.getElementById('customTimecodeGroup')!;
    
    if (timecodeFormat.value === 'custom') {
      customGroup.style.display = 'block';
    } else {
      customGroup.style.display = 'none';
    }
  }

  private displayTranscriptionFiles() {
    const listElement = document.getElementById('transcriptionList')!;
    listElement.innerHTML = '';

    this.transcriptionFiles.forEach((file, index) => {
      const fileItem = document.createElement('div');
      fileItem.className = 'file-item';
      
      // Create info container
      const fileInfo = document.createElement('div');
      fileInfo.className = 'file-item-info';
      
      // Create order number
      const orderDiv = document.createElement('div');
      orderDiv.className = 'file-item-order';
      orderDiv.textContent = (index + 1).toString();
      
      // Create file name
      const fileName = document.createElement('span');
      fileName.className = 'file-item-name';
      fileName.textContent = file.split('/').pop() || '';
      
      // Assemble info
      fileInfo.appendChild(orderDiv);
      fileInfo.appendChild(fileName);
      
      // Create actions container
      const actionsDiv = document.createElement('div');
      actionsDiv.className = 'file-item-actions';
      
      // Create up button
      const upButton = document.createElement('button');
      upButton.className = 'icon-btn';
      upButton.onclick = () => this.moveFileUp(index);
      upButton.disabled = index === 0;
      upButton.title = 'Переместить вверх';
      upButton.innerHTML = `
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"></path>
        </svg>
      `;
      
      // Create down button
      const downButton = document.createElement('button');
      downButton.className = 'icon-btn';
      downButton.onclick = () => this.moveFileDown(index);
      downButton.disabled = index === this.transcriptionFiles.length - 1;
      downButton.title = 'Переместить вниз';
      downButton.innerHTML = `
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
        </svg>
      `;
      
      // Create delete button
      const deleteButton = document.createElement('button');
      deleteButton.className = 'icon-btn';
      deleteButton.onclick = () => this.removeTranscriptionFile(index);
      deleteButton.title = 'Удалить файл';
      deleteButton.innerHTML = `
        <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
        </svg>
      `;
      
      // Assemble actions
      actionsDiv.appendChild(upButton);
      actionsDiv.appendChild(downButton);
      actionsDiv.appendChild(deleteButton);
      
      // Assemble final item
      fileItem.appendChild(fileInfo);
      fileItem.appendChild(actionsDiv);
      
      listElement.appendChild(fileItem);
    });
    
    // Update button states
    const clearAllBtn = document.getElementById('clearAllBtn') as HTMLButtonElement;
    const mergeBtn = document.getElementById('mergeBtn') as HTMLButtonElement;
    const hasFiles = this.transcriptionFiles.length > 0;
    
    clearAllBtn.disabled = !hasFiles;
    mergeBtn.disabled = !hasFiles;
  }

  private moveFileUp(index: number) {
    if (index > 0) {
      const file = this.transcriptionFiles[index];
      this.transcriptionFiles.splice(index, 1);
      this.transcriptionFiles.splice(index - 1, 0, file);
      this.displayTranscriptionFiles();
    }
  }

  private moveFileDown(index: number) {
    if (index < this.transcriptionFiles.length - 1) {
      const file = this.transcriptionFiles[index];
      this.transcriptionFiles.splice(index, 1);
      this.transcriptionFiles.splice(index + 1, 0, file);
      this.displayTranscriptionFiles();
    }
  }

  public removeTranscriptionFile(index: number) {
    this.transcriptionFiles.splice(index, 1);
    this.displayTranscriptionFiles();
  }

  public clearAllTranscriptions() {
    this.transcriptionFiles = [];
    this.displayTranscriptionFiles();
  }

  public async openFolder(filePath: string) {
    try {
      // Получить каталог, содержащий файл
      const directory = filePath.substring(0, filePath.lastIndexOf('/'));
      await this.invoke('open_folder', { path: directory });
    } catch (error) {
      console.error('Ошибка открытия папки:', error);
      // Запасной вариант: копировать путь в буфер обмена
      navigator.clipboard.writeText(filePath);
      alert('Не удалось открыть папку. Путь к файлу скопирован в буфер обмена.');
    }
  }

  public async openSegmentsFolder(folderPath: string) {
    try {
      console.log('Frontend: Открываем папку:', folderPath);
      console.log('Frontend: Длина пути:', folderPath.length);
      console.log('Frontend: Существует ли путь:', folderPath !== null && folderPath !== undefined);
      
      // Fix path for Windows - ensure double backslashes for JSON serialization
      const fixedPath = folderPath.replace(/\\/g, '\\\\');
      console.log('Frontend: Fixed path:', fixedPath);
      
      await this.invoke('open_folder', { path: fixedPath });
      console.log('Frontend: Команда open_folder выполнена успешно');
    } catch (error) {
      console.error('Ошибка открытия папки с сегментами:', error);
      // Запасной вариант: копировать путь в буфер обмена
      navigator.clipboard.writeText(folderPath);
      alert('Не удалось открыть папку. Путь скопирован в буфер обмена.');
    }
  }

  private async mergeTranscriptions() {
    if (this.transcriptionFiles.length === 0) return;

    const mergeBtn = document.getElementById('mergeBtn') as HTMLButtonElement;
    const originalText = mergeBtn.textContent;
    
    try {
      mergeBtn.disabled = true;
      mergeBtn.textContent = '🔄 Объединяем...';
      
      const outputFormat = (document.getElementById('outputFormat') as HTMLSelectElement).value;
      
      const result = await this.invoke('merge_transcriptions', {
        files: this.transcriptionFiles,
        outputFormat
      });

      console.log('Объединение завершено:', result);
      mergeBtn.textContent = '✅ Объединено!';
      (document.getElementById('exportBtn') as HTMLButtonElement).disabled = false;
      
      // Показать сообщение об успехе
      this.showMergeStatus('✅ Транскрипции успешно объединены! Готово к экспорту.', 'success');
      
      setTimeout(() => {
        mergeBtn.textContent = originalText;
        mergeBtn.disabled = false;
      }, 2000);
      
    } catch (error) {
      console.error('Ошибка объединения транскрипций:', error);
      mergeBtn.textContent = originalText;
      mergeBtn.disabled = false;
      
      const errorMessage = error instanceof Error ? error.message : 'Произошла неизвестная ошибка';
      this.showMergeStatus(`❌ Ошибка объединения транскрипций: ${errorMessage}`, 'error');
    }
  }

  private showMergeStatus(message: string, type: 'success' | 'error') {
    const statusDiv = document.createElement('div');
    statusDiv.className = `status status-${type}`;
    statusDiv.textContent = message;
    
    const mergeCard = document.querySelector('.card:has(#transcriptionList)');
    const existingStatus = mergeCard?.querySelector('.status');
    if (existingStatus) {
      existingStatus.remove();
    }
    
    mergeCard?.appendChild(statusDiv);
    
    setTimeout(() => {
      statusDiv.remove();
    }, 5000);
  }

  private async exportResults() {
    const exportBtn = document.getElementById('exportBtn') as HTMLButtonElement;
    const outputPathInput = document.getElementById('outputPath') as HTMLInputElement;
    const outputFileNameInput = document.getElementById('outputFileName') as HTMLInputElement;
    const outputFormatSelect = document.getElementById('outputFormat') as HTMLSelectElement;
    const timecodeFormatSelect = document.getElementById('timecodeFormat') as HTMLSelectElement;
    const customTimecodeFormatInput = document.getElementById('customTimecodeFormat') as HTMLInputElement;
    const includeExtendedInfoCheckbox = document.getElementById('includeExtendedInfo') as HTMLInputElement;
    
    const originalText = exportBtn.textContent;
    
    // Валидация
    if (!outputPathInput.value.trim()) {
      alert('Пожалуйста, выберите папку для сохранения');
      return;
    }
    
    if (!outputFileNameInput.value.trim()) {
      alert('Пожалуйста, укажите имя файла');
      return;
    }
    
    try {
      exportBtn.disabled = true;
      exportBtn.textContent = '📤 Экспортируем...';
      
      const result = await this.invoke('export_merged_transcription', {
        outputPath: outputPathInput.value,
        fileName: outputFileNameInput.value,
        outputFormat: outputFormatSelect.value,
        timecodeFormat: timecodeFormatSelect.value,
        customTimecodeFormat: timecodeFormatSelect.value === 'custom' ? customTimecodeFormatInput.value : null,
        includeExtendedInfo: includeExtendedInfoCheckbox.checked
      });
      console.log('Экспорт завершен:', result);
      
      exportBtn.textContent = '✅ Экспортировано!';
      
      // Показать сообщение об успехе
      const message = result?.path 
        ? `✅ Файл успешно экспортирован в: ${result.path}`
        : '✅ Экспорт завершен успешно!';
      this.showExportStatus(message, 'success');
      
      setTimeout(() => {
        exportBtn.textContent = originalText;
        exportBtn.disabled = false;
      }, 2000);
      
    } catch (error) {
      console.error('Ошибка экспорта:', error);
      exportBtn.textContent = originalText;
      exportBtn.disabled = false;
      
      const errorMessage = error instanceof Error ? error.message : 'Произошла неизвестная ошибка';
      this.showExportStatus(`❌ Ошибка экспорта файла: ${errorMessage}`, 'error');
    }
  }

  private showExportStatus(message: string, type: 'success' | 'error') {
    const statusDiv = document.createElement('div');
    statusDiv.className = `status status-${type}`;
    statusDiv.textContent = message;
    
    const exportCard = document.querySelector('.card:has(#exportBtn)');
    const existingStatus = exportCard?.querySelector('.status');
    if (existingStatus) {
      existingStatus.remove();
    }
    
    exportCard?.appendChild(statusDiv);
    
    setTimeout(() => {
      statusDiv.remove();
    }, 7000);
  }

  private async setupTauriFileDrop() {
    console.log('🔧 Setting up Tauri file drop...');
    
    // Listen for file drop events from Tauri
    await this.listen('tauri://file-drop', (event: any) => {
      console.log('🎯 Tauri file drop event received!', event);
      
      const files = event.payload as string[];
      console.log('📁 Dropped files:', files);
      
      if (files && files.length > 0) {
        // Process each file based on its extension
        const mediaFiles: string[] = [];
        const transcriptionFiles: string[] = [];
        
        files.forEach(filePath => {
          const extension = filePath.split('.').pop()?.toLowerCase();
          console.log('📋 Processing file:', filePath, 'extension:', extension);
          
          // Check if it's a media file
          const mediaExtensions = ['mp4', 'avi', 'mov', 'mkv', 'webm', 'flv', 'wmv', 'mp3', 'wav', 'aac', 'flac', 'ogg', 'm4a', 'wma', 'opus'];
          const transcriptionExtensions = ['txt', 'srt', 'md'];
          
          if (mediaExtensions.includes(extension || '')) {
            mediaFiles.push(filePath);
          } else if (transcriptionExtensions.includes(extension || '')) {
            transcriptionFiles.push(filePath);
          }
        });
        
        // Handle media files
        if (mediaFiles.length > 0) {
          console.log('🎵 Processing media files:', mediaFiles);
          this.handleMediaFiles(mediaFiles);
        }
        
        // Handle transcription files  
        if (transcriptionFiles.length > 0) {
          console.log('📝 Processing transcription files:', transcriptionFiles);
          this.handleTranscriptionFiles(transcriptionFiles);
        }
        
        if (mediaFiles.length === 0 && transcriptionFiles.length === 0) {
          this.showFileErrorState('Неподдерживаемый формат файла. Используйте медиа или текстовые файлы.');
        }
      }
    });
    
    console.log('✅ Tauri file drop setup complete');
  }


  private handleMediaFiles(filePaths: string[]) {
    console.log('🎵 Media file handler called with files:', filePaths.length);
    
    if (filePaths.length > 0) {
      const filePath = filePaths[0]; // Take first file
      console.log('🎵 Processing media file:', filePath);
      
      // Show loading state
      this.showFileLoadingState(true);
      
      this.selectedFile = filePath;
      this.displayFileInfo(filePath);
      (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
    }
  }

  private showFileLoadingState(loading: boolean) {
    const dropZone = document.getElementById('fileDropZone')!;
    const dropIcon = document.getElementById('dropIcon')!;
    const loadingSpinner = document.getElementById('loadingSpinner')!;
    const dropText = document.getElementById('dropText')!;
    const selectBtn = document.getElementById('selectFileBtn')! as HTMLButtonElement;
    
    if (loading) {
      dropZone.classList.add('loading');
      dropIcon.style.display = 'none';
      loadingSpinner.style.display = 'block';
      dropText.textContent = 'Анализ файла...';
      selectBtn.disabled = true;
    } else {
      dropZone.classList.remove('loading');
      dropIcon.style.display = 'block';
      loadingSpinner.style.display = 'none';
      dropText.textContent = 'Перетащите сюда ваш аудио/видео файл';
      selectBtn.disabled = false;
    }
  }

  private showFileSuccessState() {
    const dropZone = document.getElementById('fileDropZone')!;
    const dropText = document.getElementById('dropText')!;
    
    dropZone.classList.remove('loading', 'error');
    dropZone.classList.add('success');
    dropText.textContent = '✓ Файл успешно загружен';
    
    // Remove success state after animation
    setTimeout(() => {
      dropZone.classList.remove('success');
      dropText.textContent = 'Перетащите сюда ваш аудио/видео файл';
    }, 2000);
  }

  private showFileErrorState(message: string) {
    const dropZone = document.getElementById('fileDropZone')!;
    const dropText = document.getElementById('dropText')!;
    
    dropZone.classList.remove('loading', 'success');
    dropZone.classList.add('error');
    dropText.textContent = message;
    
    // Remove error state after animation
    setTimeout(() => {
      dropZone.classList.remove('error');
      dropText.textContent = 'Перетащите сюда ваш аудио/видео файл';
    }, 3000);
  }

  private handleTranscriptionFiles(filePaths: string[]) {
    console.log('📝 Transcription files handler called with files:', filePaths.length);
    
    if (filePaths.length > 0) {
      // Sort files automatically by numeric sequence
      this.transcriptionFiles = this.sortTranscriptionFiles(filePaths);
      this.displayTranscriptionFiles();
      this.setDefaultOutputPath();
      (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = false;
    }
  }

  private sortTranscriptionFiles(filePaths: string[]): string[] {
    return filePaths.sort((a, b) => {
      // Extract filename without path
      const fileA = a.split(/[\\/]/).pop() || '';
      const fileB = b.split(/[\\/]/).pop() || '';
      
      // Try to extract numbers from filenames
      const numberA = this.extractSequenceNumber(fileA);
      const numberB = this.extractSequenceNumber(fileB);
      
      // If both have numbers, sort by number
      if (numberA !== null && numberB !== null) {
        return numberA - numberB;
      }
      
      // If only one has a number, prioritize it
      if (numberA !== null && numberB === null) return -1;
      if (numberA === null && numberB !== null) return 1;
      
      // If neither has numbers, sort alphabetically
      return fileA.localeCompare(fileB, 'ru', { numeric: true });
    });
  }

  private extractSequenceNumber(filename: string): number | null {
    // Remove file extension
    const nameWithoutExt = filename.replace(/\.[^.]*$/, '');
    
    // Pattern 1: Pure numbers at the end (e.g., "1.txt", "part2.srt")
    let match = nameWithoutExt.match(/(\d+)$/);
    if (match) {
      return parseInt(match[1], 10);
    }
    
    // Pattern 2: Numbers with common separators (e.g., "part_1.txt", "часть-2.srt", "segment 3.md")
    match = nameWithoutExt.match(/[_\-\s](\d+)$/);
    if (match) {
      return parseInt(match[1], 10);
    }
    
    // Pattern 3: Numbers after common words (case insensitive)
    match = nameWithoutExt.match(/(?:part|часть|segment|сегмент|file|файл|chapter|глава)[_\-\s]*(\d+)$/i);
    if (match) {
      return parseInt(match[1], 10);
    }
    
    // Pattern 4: First number found in filename (fallback)
    match = nameWithoutExt.match(/(\d+)/);
    if (match) {
      return parseInt(match[1], 10);
    }
    
    return null;
  }

}