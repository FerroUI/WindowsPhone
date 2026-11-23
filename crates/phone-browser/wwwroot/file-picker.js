// One user-initiated picker, shared by documents, media and PIM adapters.
export function chooseFiles(accept, multiple = false) {
  return new Promise(resolve => {
    const input = document.createElement('input');
    input.type = 'file'; input.accept = accept; input.multiple = multiple;
    input.style.display = 'none';
    let complete = false;
    const done = files => {
      if (complete) return;
      complete = true; input.remove(); resolve(files);
    };
    input.addEventListener('change', () => done([...input.files]), {once: true});
    input.addEventListener('cancel', () => done([]), {once: true});
    document.body.append(input); input.click();
  });
}
