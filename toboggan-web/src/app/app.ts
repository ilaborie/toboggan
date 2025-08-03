import { KeyboardModule } from "./keyboard";
import { TobogganNavigationElement } from "../components/navigation";
import { TobogganToastElement } from "../components/toast";
import { Command, CommandHandler, State, Talk } from "../types";
import { AppConfig } from "../config";
import { TobogganSlideElement } from "../components";
import { CommunicationCallbacks, CommunicationService, ConnectionStatus, formatConnectionStatus } from "./communication";
import { SlidesApiService } from "./api";

export class TobogganApp implements CommunicationCallbacks, CommandHandler {
    private readonly navigationElement: TobogganNavigationElement;
    private readonly slideElement: TobogganSlideElement;
    private readonly toastElement: TobogganToastElement;

    private readonly keyboardModule: KeyboardModule;
    private readonly communicationService: CommunicationService;
    private readonly slidesApi: SlidesApiService;

    private talk: Talk | null = null;
    private state: State | null = null;

    constructor(appConfig: AppConfig) {
        const { clientId, apiBaseUrl, websocket } = appConfig;
        this.navigationElement = getRequireElement("toboggan-navigation");
        this.slideElement = getRequireElement("toboggan-slide");
        this.toastElement = getRequireElement("toboggan-toast");

        this.keyboardModule = new KeyboardModule(this);
        this.slidesApi = new SlidesApiService(apiBaseUrl);
        this.communicationService = new CommunicationService(clientId, websocket, this);

        this.start();
    }

    private async start() {
        this.communicationService.connect();
        this.keyboardModule.start();
        this.talk = await this.slidesApi.getTalk();

        if (this.talk) {
            this.navigationElement.talk = this.talk;
            this.navigationElement.slideCount = this.talk.titles.length;
        }
    }

    public onCommand(command: Command): void {
        this.communicationService.sendCommand(command);
    }

    onConnectionStatusChange(status: ConnectionStatus) {
        console.log('📡', { status });
        this.navigationElement.connectionStatus = status;

        const message = formatConnectionStatus(status);
        switch (status.status) {
            case "connecting":
                this.toastElement.toast("info", message);
                break;
            case "connected":
                if (!status.latency) {
                    this.toastElement.toast("info", message);
                    this.communicationService.register();
                }
                break;
            case "reconnecting":
                this.toastElement.toast("warning", message);
                console.log();
                break;
            case "closed":
                console.log('🚪 Closed');
                break;
            case "error":
                this.onError(status.message);
                break;
        }
    }

    async onStateChange(state: State) {
        console.log('🗽', { state });
        this.state = state;

        this.navigationElement.state = state.state;
        this.navigationElement.slideCurrent = state.current;
        this.navigationElement.duration = state.total_duration;

        if (state.state === 'Done') {
            this.toastElement.toast('success', '🎉 Done');
        }

        // Load and display slide
        await this.loadCurrentSlide();
    }

    onError(message: string) {
        console.log('🚨', { message });
        console.error("PresentationController error:", message);
        this.toastElement.toast('error', message);
    }

    private async loadCurrentSlide(): Promise<void> {
        const currentSlide = this.state?.current;
        if (typeof currentSlide !== 'number') return;

        try {
            const slide = await this.slidesApi.getSlide(currentSlide);
            this.slideElement.slide = slide;
            this.navigationElement.slideCurrent = currentSlide + 1;
        } catch (error) {
            this.toastElement.toast("error", `Failed to load slide:${error}`);
        }
    }

    public dispose(): void {
        this.keyboardModule.dispose();
        this.communicationService.dispose();
    }
}

const getRequireElement = <E extends HTMLElement>(selector: string): E => {
    const result = document.querySelector(selector);
    if (!result) {
        throw new Error(`missing ${selector} element`);
    }
    return result as E;
} 