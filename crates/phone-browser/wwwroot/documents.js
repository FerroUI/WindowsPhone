import {chooseFiles} from './media.js';

const MAX_IMPORT_BYTES = 8 * 1024 * 1024;
const EXTENSIONS = '.docx,.xlsx,.pptx,.txt,.csv';

export async function importDocuments(runtime, folder, report) {
  const files = await chooseFiles(EXTENSIONS, true);
  let imported = 0;
  const failures = [];
  for (const file of files) {
    try {
      if (file.size > MAX_IMPORT_BYTES) throw new Error('Choose a document smaller than 8 MB.');
      runtime.importOffice(file.name, new Uint8Array(await file.arrayBuffer()), folder ?? 0);
      imported++;
    } catch (error) {
      failures.push(`${file.name}: ${error.message || error}`);
    }
  }
  if (failures.length) report(`${imported ? `Imported ${imported}. ` : ''}${failures.join('\n')}`);
  else if (imported) report(`Imported ${imported} ${imported === 1 ? 'document' : 'documents'}.`);
}

export function exportDocument(runtime, id) {
  const result = JSON.parse(runtime.exportOffice(id));
  const file = new Blob([Uint8Array.from(result.bytes)], {type: result.mime});
  const url = URL.createObjectURL(file);
  const link = document.createElement('a');
  link.href = url;
  link.download = result.name;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}
