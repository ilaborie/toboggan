import type { TobogganSlideElement } from "../components";
import type { TobogganNavigationElement } from "../components/navigation";
import type { TobogganToastElement } from "../components/toast";
import type { AppConfig } from "../config";
import type { Command, CommandHandler, State, Talk } from "../types";
import { getRequireElement } from "../utils/dom";
import { SlidesApiService } from "./api";
import {
  type CommunicationCallbacks,
  CommunicationService,
  type ConnectionStatus,
  formatConnectionStatus,
} from "./communication";
import { KeyboardModule } from "./keyboard";

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
      this.navigationElement.addEventListener("command", (cmdEvent) =>
        this.onCommand(cmdEvent.detail)
      );

      this.navigationElement.talk = this.talk;
      this.navigationElement.slideCount = this.talk.titles.length;
    }
  }

  public onCommand(command: Command): void {
    this.communicationService.sendCommand(command);
  }

  onConnectionStatusChange(status: ConnectionStatus) {
    // console.log("📡", { status });
    this.navigationElement.connectionStatus = status;

    const message = formatConnectionStatus(status);
    switch (status.status) {
      case "connecting":
        this.toastElement.toast("info", message);
        break;
      case "connected":
        this.toastElement.toast("success", message);
        this.communicationService.register();
        break;
      case "latency":
        console.debug(message);
        break;
      case "reconnecting":
        this.toastElement.toast("warning", message);
        break;
      case "closed":
        this.toastElement.toast("error", message);
        break;
      case "error":
        this.onError(status.message);
        break;
    }
  }

  async onStateChange(state: State) {
    // console.log("🗽", { state });
    this.state = state;

    this.navigationElement.state = state.state;
    this.navigationElement.slideCurrent = state.current;
    this.navigationElement.duration = state.total_duration;

    if (state.state === "Done") {
      this.toastElement.toast("success", "🎉 Done");
    }

    // Load and display slide
    await this.loadCurrentSlide();
  }

  onError(message: string) {
    console.error("PresentationController error:", message);
    this.toastElement.toast("error", message);
  }

  private async loadCurrentSlide(): Promise<void> {
    const currentSlide = this.state?.current;
    if (typeof currentSlide !== "number") return;

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
