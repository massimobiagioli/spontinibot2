import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import * as adminApi from '../../../services/adminApi';
import UploadDropzone from '../UploadDropzone.vue';

function selectFile(wrapper: ReturnType<typeof mount>, file: File): void {
  const input = wrapper.find('input[type="file"]').element as HTMLInputElement;
  Object.defineProperty(input, 'files', {
    value: [file],
    configurable: true,
  });
  input.dispatchEvent(new Event('change'));
}

describe('UploadDropzone', () => {
  it('uploads a file, shows the preview, confirms, and shows the chunk count', async () => {
    const uploadSpy = vi.spyOn(adminApi, 'uploadDocument').mockResolvedValue({
      token: 'abc',
      preview_url: '/admin/api/upload/preview/abc',
    });
    const previewSpy = vi
      .spyOn(adminApi, 'getUploadPreview')
      .mockResolvedValue({
        extracted_text: 'contenuto estratto',
        format: 'txt',
        byte_size: 19,
        section: 'news',
        filename: 'doc.txt',
        metadata: { category: null, tags: null, trust_score: null },
        chunk_count_estimate: 1,
      });
    const confirmSpy = vi.spyOn(adminApi, 'confirmUpload').mockResolvedValue({
      document_ids: [1],
      chunk_count: 1,
    });

    const wrapper = mount(UploadDropzone, { props: { sectionName: 'news' } });
    const file = new File(['contenuto'], 'doc.txt', { type: 'text/plain' });
    selectFile(wrapper, file);

    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(uploadSpy).toHaveBeenCalledWith(file, 'news', {
      category: undefined,
      tags: undefined,
      trustScore: undefined,
    });
    expect(previewSpy).toHaveBeenCalledWith('abc');
    expect(wrapper.text()).toContain('doc.txt');
    expect(wrapper.text()).toContain('contenuto estratto');

    await wrapper.findAll('button')[0]?.trigger('click');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(confirmSpy).toHaveBeenCalledWith('abc');
    expect(wrapper.text()).toContain('1 blocchi creati');
  });

  it('shows an honest error message when the upload fails', async () => {
    vi.spyOn(adminApi, 'uploadDocument').mockRejectedValue(
      new adminApi.AdminApiError(400, 'unsupported file format'),
    );

    const wrapper = mount(UploadDropzone, { props: { sectionName: 'news' } });
    const file = new File(['x'], 'doc.exe', {
      type: 'application/x-msdownload',
    });
    selectFile(wrapper, file);

    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('unsupported file format');
  });

  it('shows an honest error message when confirming the upload fails', async () => {
    vi.spyOn(adminApi, 'uploadDocument').mockResolvedValue({
      token: 'abc',
      preview_url: '/admin/api/upload/preview/abc',
    });
    vi.spyOn(adminApi, 'getUploadPreview').mockResolvedValue({
      extracted_text: 'contenuto',
      format: 'txt',
      byte_size: 9,
      section: 'news',
      filename: 'doc.txt',
      metadata: { category: null, tags: null, trust_score: null },
      chunk_count_estimate: 1,
    });
    vi.spyOn(adminApi, 'confirmUpload').mockRejectedValue(
      new adminApi.AdminApiError(404, 'preview token not found'),
    );

    const wrapper = mount(UploadDropzone, { props: { sectionName: 'news' } });
    const file = new File(['contenuto'], 'doc.txt', { type: 'text/plain' });
    selectFile(wrapper, file);

    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    await wrapper.findAll('button')[0]?.trigger('click');
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain('preview token not found');
    // falls back to the preview phase (not stuck on "confirming"), so the
    // confirm/cancel actions are still available to retry or bail out
    expect(wrapper.findAll('button')).toHaveLength(2);
  });

  it('annulla resets the form without confirming the upload', async () => {
    vi.spyOn(adminApi, 'uploadDocument').mockResolvedValue({
      token: 'abc',
      preview_url: '/admin/api/upload/preview/abc',
    });
    vi.spyOn(adminApi, 'getUploadPreview').mockResolvedValue({
      extracted_text: 'contenuto',
      format: 'txt',
      byte_size: 9,
      section: 'news',
      filename: 'doc.txt',
      metadata: { category: null, tags: null, trust_score: null },
      chunk_count_estimate: 1,
    });
    const confirmSpy = vi.spyOn(adminApi, 'confirmUpload');

    const wrapper = mount(UploadDropzone, { props: { sectionName: 'news' } });
    const file = new File(['contenuto'], 'doc.txt', { type: 'text/plain' });
    selectFile(wrapper, file);

    await wrapper.find('form').trigger('submit');
    await Promise.resolve();
    await Promise.resolve();
    await wrapper.vm.$nextTick();

    const buttons = wrapper.findAll('button');
    await buttons[1]?.trigger('click');
    await wrapper.vm.$nextTick();

    expect(confirmSpy).not.toHaveBeenCalled();
    expect(wrapper.find('input[type="file"]').exists()).toBe(true);
  });
});
