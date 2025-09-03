import './styles/main.css';

// Add loading indicator
console.log('Загрузка Помощника транскрипции...');
document.addEventListener('DOMContentLoaded', () => {
  console.log('DOM загружен, запуск приложения...');
  initApp();
});

async function initApp() {
  try {
    const { invoke } = await import('@tauri-apps/api/tauri');
    const { open } = await import('@tauri-apps/api/dialog');
    const { listen } = await import('@tauri-apps/api/event');
    
    console.log('API Tauri успешно загружены');
    const app = new TranscriptionAssistant(invoke, open, listen);
    (window as any).app = app;
  } catch (error) {
    console.error('Не удалось загрузить API Tauri:', error);
    alert('Ошибка загрузки приложения: ' + error);
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

    // Временно отключено перетаскивание до исправления интеграции с Tauri
    // TODO: Реализовать корректные события перетаскивания файлов в Tauri
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
          name: 'Медиа файлы',
          extensions: ['mp4', 'avi', 'mov', 'mkv', 'webm', 'flv', 'wmv', 'mp3', 'wav', 'aac', 'flac', 'ogg', 'm4a', 'wma', 'opus']
        }]
      });

      if (selected && typeof selected === 'string') {
        this.selectedFile = selected;
        await this.displayFileInfo(selected);
        (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
      }
    } catch (error) {
      console.error('Ошибка выбора файла:', error);
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
      console.error('Ошибка получения информации о файле:', error);
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

  private onProcessingComplete(result: any) {
    console.log('Обработка завершена:', result);
    this.updateProgress(100, 'Обработка завершена!');
    (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
    
    // Показать раздел результатов и отобразить обработанные сегменты
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
      title.textContent = `✅ Создано ${result.segments.length} аудио сегментов:`;
      segmentsContainer.appendChild(title);

      result.segments.forEach((segment: any, index: number) => {
        const segmentItem = document.createElement('div');
        segmentItem.className = 'segment-item';
        segmentItem.innerHTML = `
          <div class="segment-info">
            <strong>Segment ${index + 1}</strong>
            <span class="segment-duration">${segment.duration || 'Длительность неизвестна'}</span>
            <span class="segment-path">${segment.path || 'Путь неизвестен'}</span>
          </div>
          <div class="segment-actions">
            <button onclick="navigator.clipboard.writeText('${segment.path}')">📋 Копировать путь</button>
            <button onclick="app.openFolder('${segment.path}')">📁 Открыть папку</button>
          </div>
        `;
        segmentsContainer.appendChild(segmentItem);
      });

      resultsDiv.appendChild(segmentsContainer);
    } else {
      resultsDiv.innerHTML = '<p>❌ Сегменты не были созданы. Проверьте логи обработки.</p>';
    }
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
        this.transcriptionFiles = selected;
        this.displayTranscriptionFiles();
        (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = false;
      }
    } catch (error) {
      console.error('Ошибка выбора файлов транскрипции:', error);
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
        <button onclick="app.removeTranscriptionFile(${index})">Удалить</button>
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
      exportBtn.textContent = '📤 Экспортируем...';
      
      const result = await this.invoke('export_merged_transcription');
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
      console.log('Перетащенный файл:', file.name);
      
      try {
        // В Tauri перетащенные файлы должны предоставлять пути к файлам через file.path
        const filePath = (file as any).path;
        if (filePath) {
          this.selectedFile = filePath;
          await this.displayFileInfo(filePath);
          (document.getElementById('startProcessingBtn') as HTMLButtonElement).disabled = false;
        } else {
          console.error('Не удалось получить путь к перетащенному файлу');
          alert('Ошибка: Не удалось получить путь к файлу. Попробуйте использовать кнопку "Выбрать файл".');
        }
      } catch (error) {
        console.error('Ошибка обработки перетащенного файла:', error);
        alert('Ошибка обработки перетащенного файла: ' + error);
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
            console.warn(`Не удалось получить путь к файлу: ${file.name}`);
          }
        }
      }
      
      if (filePaths.length > 0) {
        this.transcriptionFiles = filePaths;
        this.displayTranscriptionFiles();
        (document.getElementById('mergeBtn') as HTMLButtonElement).disabled = false;
      } else if (files.length > 0) {
        alert('Не удалось получить пути к перетащенным файлам. Попробуйте использовать кнопку "Выбрать файлы".');
      }
    } catch (error) {
      console.error('Ошибка обработки перетащенных файлов транскрипции:', error);
      alert('Ошибка обработки перетащенных файлов: ' + error);
    }
  }
}