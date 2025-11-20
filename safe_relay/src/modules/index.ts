export default interface Module {
    init(): Promise<void>;

    tick(): Promise<void>;
}