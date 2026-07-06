/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Modal } from './Modal';

describe('Modal', () => {
  it('renders nothing when closed', () => {
    const { container } = render(
      <Modal isOpen={false} onClose={jest.fn()} labelledBy="modal-title">
        <h2 id="modal-title">Title</h2>
      </Modal>
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('exposes dialog semantics and closes on Escape', () => {
    const onClose = jest.fn();
    render(
      <Modal isOpen onClose={onClose} labelledBy="modal-title">
        <h2 id="modal-title">Title</h2>
        <button type="button">Action</button>
      </Modal>
    );

    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAttribute('aria-labelledby', 'modal-title');

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('traps Tab focus within the dialog', async () => {
    const user = userEvent.setup();
    render(
      <Modal isOpen onClose={jest.fn()} labelledBy="modal-title">
        <h2 id="modal-title">Title</h2>
        <button type="button">First</button>
        <button type="button">Last</button>
      </Modal>
    );

    const first = screen.getByText('First');
    const last = screen.getByText('Last');
    expect(document.activeElement).toBe(first);

    await user.tab();
    expect(document.activeElement).toBe(last);

    await user.tab();
    expect(document.activeElement).toBe(first);
  });
});
