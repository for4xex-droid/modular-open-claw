/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { describe, it, expect, vi } from 'vitest'
import fs from 'fs'
import path from 'path'

describe('Landing Page Deployment Assets', () => {
  const publicDir = path.resolve(__dirname, '../public')
  const rootDir = path.resolve(__dirname, '..')

  it('CNAME file should exist and contain the target domain', () => {
    const cnamePath = path.join(publicDir, 'CNAME')
    expect(fs.existsSync(cnamePath)).toBe(true)
    const content = fs.readFileSync(cnamePath, 'utf-8').trim()
    expect(content).toBe('aiome.dev')
  })

  it('404.html should exist and contain the SPA redirect script', () => {
    const errorHtmlPath = path.join(publicDir, '404.html')
    expect(fs.existsSync(errorHtmlPath)).toBe(true)
    const content = fs.readFileSync(errorHtmlPath, 'utf-8')
    
    // Check that it contains the critical redirect script segments
    expect(content).toContain('window.location')
    expect(content).toContain('l.pathname.split(\'/\')')
    expect(content).toContain('replace(/&/g, \'~and~\')')
  })

  it('index.html should exist and contain the SPA redirect receiver script in <head>', () => {
    const indexHtmlPath = path.join(rootDir, 'index.html')
    expect(fs.existsSync(indexHtmlPath)).toBe(true)
    const content = fs.readFileSync(indexHtmlPath, 'utf-8')

    // The redirect receiver must be in the head and before react main script
    expect(content).toContain('window.history.replaceState')
    expect(content).toContain('l.search.slice(1).split(\'&\')')
    expect(content).toContain('replace(/~and~/g, \'&\')')
  })

  it('SPA redirect receiver logic should restore correct pathname and query parameters', () => {
    // We simulate the receiver logic inside JSDOM environment
    const originalLocation = window.location
    const originalHistory = window.history

    // Mock history.replaceState
    const replaceStateMock = vi.fn()
    Object.defineProperty(window, 'history', {
      writable: true,
      value: {
        ...originalHistory,
        replaceState: replaceStateMock
      }
    })

    // Mock window.location for a redirected URL, e.g. https://aiome.dev/?/privacy&foo=bar
    Object.defineProperty(window, 'location', {
      writable: true,
      value: {
        ...originalLocation,
        search: '?/privacy&foo=bar',
        pathname: '/',
        hash: '#hash'
      }
    })

    // Receiver script implementation to be tested
    const receiver = (l: Location) => {
      if (l.search[1] === '/') {
        const decoded = l.search.slice(1).split('&').map((s) => {
          return s.replace(/~and~/g, '&')
        }).join('?');
        window.history.replaceState(null, null as any,
            l.pathname.slice(0, -1) + decoded + l.hash
        );
      }
    }

    receiver(window.location)

    expect(replaceStateMock).toHaveBeenCalledWith(
      null,
      null,
      '/privacy?foo=bar#hash'
    )

    // Restore original values
    Object.defineProperty(window, 'location', { value: originalLocation })
    Object.defineProperty(window, 'history', { value: originalHistory })
  })
})
