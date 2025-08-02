
import { KeyboardModule } from "./keyboard";
import { TobogganNavigationElement } from "../components/navigation";
import { TobogganToastElement } from "../components/toast";
import { Command, SlideId, State, Talk } from "../types";
import { AppConfig } from "../config";
import { renderContent, TobogganSlideElement } from "../components";
import { CommunicationCallbacks, CommunicationService, ConnectionStatus, formatConnectionStatus } from "./communication";
import { SlidesApiService } from "./api";

export class TobogganApp implements CommunicationCallbacks {
    private readonly elements: PresentationElements;
    private readonly keyboardModule: KeyboardModule;
    private readonly communicationService: CommunicationService;
    private readonly slidesApi: SlidesApiService;

    private talk: Talk | null = null;
    private currentSlide: SlideId | null = null;

    constructor(appConfig: AppConfig) {
        this.elements = {
            navigationElement: getRequireElement<TobogganNavigationElement>("toboggan-navigation"),
            slideElement: getRequireElement<TobogganSlideElement>("toboggan-slide"),
            toastElement: getRequireElement<TobogganToastElement>("toboggan-toast"),
        };
        const clientId = crypto.randomUUID();
        this.keyboardModule = new KeyboardModule(this);
        this.slidesApi = new SlidesApiService(appConfig.apiBaseUrl);
        this.communicationService = new CommunicationService(clientId, appConfig.websocket, this);

        this.start();
    }

    private async start() {
        this.communicationService.connect();
        this.keyboardModule.start();
        this.talk = await this.slidesApi.getTalk();

        if (this.talk) {
            this.elements.navigationElement.talk = this.talk;
            this.elements.navigationElement.slideCount = this.talk.titles.length;
        }
    }

    /**
     * Handle commands from keyboard and navigation modules
    */
    public onCommand(command: Command): void {
        this.communicationService.sendCommand(command);
    }

    onConnectionStatusChange(status: ConnectionStatus) {
        this.elements.navigationElement.connectionStatus = status;
        this.elements.toastElement.toast("info", formatConnectionStatus(status));

        if (status.status === "connected") {
            this.communicationService.register();
        }
    }

    async onStateChange(state: State) {
        this.elements.navigationElement.state = state.state;
        this.elements.navigationElement.slideCurrent = state.current;
        this.elements.navigationElement.duration = state.total_duration;

        if (state.state === 'Done') {
            this.elements.toastElement.toast('success', '🎉 Done');
        }

        this.currentSlide = state.current;

        // Load and display slide
        await this.loadCurrentSlide();
    }

    onError(message: string) {
        console.error("PresentationController error:", message);
        this.elements.toastElement.toast('error', message);
    }

    private async loadCurrentSlide(): Promise<void> {
        if (this.currentSlide === null) return;

        try {
            const slide = await this.slidesApi.getSlide(this.currentSlide);
            this.elements.navigationElement.slideCurrent = (this.currentSlide !== null) ? this.currentSlide + 1 : null;
            this.elements.slideElement.slide = slide;
        } catch (error) {
            this.elements.toastElement.toast("error", `Failed to load slide:${error}`);
        }
    }
    /**
     * Dispose of the application resources
     */
    public dispose(): void {
        this.keyboardModule.dispose();
        this.communicationService.dispose();
    }
}

export interface PresentationElements {
    navigationElement: TobogganNavigationElement;
    slideElement: TobogganSlideElement;
    toastElement: TobogganToastElement;
}

const getRequireElement = <E extends HTMLElement>(selector: string): E => {
    const result = document.querySelector(selector);
    if (!result) {
        throw new Error(`missing ${selector} element`);
    }
    return result as E;
} 