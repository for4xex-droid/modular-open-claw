/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import HomePage from './HomePage';

// Mock dependencies
jest.mock('../../i18n', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

jest.mock('../../hooks/useAvatarState', () => ({
    useAvatarState: () => 'idle'
}));

jest.mock('../../hooks/useDisplayMode', () => ({
    useDisplayMode: () => ({ mode: 'vrm' })
}));

jest.mock('../../hooks/AvatarContext', () => ({
    useAvatarCharacter: () => ({ getAssetPath: () => '/mock-model.vrm' })
}));

// Mock sub-components
jest.mock('./CharacterPanel', () => (props: any) => (
    <div data-testid="character-panel">
        <button onClick={props.onOpenViewer} data-testid="open-viewer-btn">Open Viewer</button>
    </div>
));
jest.mock('./StoryFlow', () => () => <div data-testid="story-flow"></div>);
jest.mock('./AvatarViewerModal', () => (props: any) => 
    props.isOpen ? <div data-testid="avatar-viewer-modal"></div> : null
);

// Mock lazy loaded components
jest.mock('../TreasureBox', () => ({ TreasureBox: () => <div data-testid="treasure-box"></div> }));
jest.mock('../VoiceStore', () => () => <div data-testid="voice-store"></div>);
jest.mock('../ArtifactVault', () => () => <div data-testid="artifact-vault"></div>);
jest.mock('../CommuneDialogueView', () => () => <div data-testid="commune-dialogue-view"></div>);
jest.mock('../BiotopeView', () => () => <div data-testid="biotope-view"></div>);
jest.mock('../GraphView', () => () => <div data-testid="graph-view"></div>);
jest.mock('../CausalVisualizer', () => () => <div data-testid="causal-visualizer"></div>);
jest.mock('../Timeline', () => () => <div data-testid="timeline"></div>);
jest.mock('../DemoView', () => () => <div data-testid="demo-view"></div>);
jest.mock('../SettingsPage', () => () => <div data-testid="settings-page"></div>);
jest.mock('../ImmuneSystem', () => () => <div data-testid="immune-system"></div>);
jest.mock('../SkillVault', () => () => <div data-testid="skill-vault"></div>);
jest.mock('../LoraTrainingView', () => () => <div data-testid="lora-training-view"></div>);
jest.mock('../ExpressionPipeline', () => () => <div data-testid="expression-pipeline"></div>);
jest.mock('../DiagnosticsHistory', () => () => <div data-testid="diagnostics-history"></div>);

describe('HomePage Component', () => {
    const mockStats = {
        level: 1,
        energy: 100,
        credits: 1000
    } as any;

    it('renders the default home tab correctly', async () => {
        render(<HomePage stats={mockStats} />);
        
        expect(screen.getByTestId('character-panel')).toBeTruthy();
        
        // Wait for lazy-loaded TreasureBox and StoryFlow
        await waitFor(() => {
            expect(screen.getByTestId('treasure-box')).toBeTruthy();
            expect(screen.getByTestId('story-flow')).toBeTruthy();
        });
        
        expect(screen.getByText('home.mainTab.home')).toBeTruthy();
        expect(screen.getByText('home.mainTab.shop')).toBeTruthy();
    });

    it('changes to Shop tab and renders shop content', async () => {
        render(<HomePage stats={mockStats} />);
        
        fireEvent.click(screen.getByText('home.mainTab.shop'));
        
        await waitFor(() => {
            expect(screen.getByText('home.tab.store')).toBeTruthy();
            expect(screen.getByText('home.tab.collection')).toBeTruthy();
            expect(screen.getByTestId('voice-store')).toBeTruthy();
        });
    });

    it('changes to World tab and hides CharacterPanel', async () => {
        render(<HomePage stats={mockStats} />);
        
        fireEvent.click(screen.getByText('home.mainTab.world'));
        
        await waitFor(() => {
            expect(screen.queryByTestId('character-panel')).toBeNull();
            expect(screen.getByTestId('biotope-view')).toBeTruthy();
        });
    });

    it('changes to Settings tab and renders settings content', async () => {
        render(<HomePage stats={mockStats} />);
        
        fireEvent.click(screen.getByText('home.mainTab.settings'));
        
        await waitFor(() => {
            expect(screen.queryByTestId('character-panel')).toBeNull();
            expect(screen.getByTestId('settings-page')).toBeTruthy();
            expect(screen.getByText('home.tab.general')).toBeTruthy();
            expect(screen.getByText('home.tab.security')).toBeTruthy();
        });
    });

    it('opens AvatarViewerModal when clicking on CharacterPanel viewer button', async () => {
        render(<HomePage stats={mockStats} />);
        
        expect(screen.queryByTestId('avatar-viewer-modal')).toBeNull();
        
        fireEvent.click(screen.getByTestId('open-viewer-btn'));
        
        expect(screen.getByTestId('avatar-viewer-modal')).toBeTruthy();
    });
});
