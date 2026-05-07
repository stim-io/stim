export type RendererEventEnvelope<
  Namespace extends string = string,
  EventKey extends string = string,
  Payload = unknown,
> = {
  namespace: Namespace;
  eventkey: EventKey;
  payload: Payload;
  event_id: string;
  emitted_at: string;
};

export type RendererEventHandler<Event extends RendererEventEnvelope> = (
  event: Event,
) => void;

export type RendererEventBus = {
  dispatch<Event extends RendererEventEnvelope>(event: Event): void;
  subscribe<Event extends RendererEventEnvelope>(
    handler: RendererEventHandler<Event>,
  ): () => void;
  subscribeNamespace<Event extends RendererEventEnvelope>(
    namespace: Event["namespace"],
    handler: RendererEventHandler<Event>,
  ): () => void;
};

export function createRendererEventBus(): RendererEventBus {
  const handlers = new Set<RendererEventHandler<RendererEventEnvelope>>();

  function dispatch<Event extends RendererEventEnvelope>(event: Event) {
    for (const handler of handlers) {
      handler(event);
    }
  }

  function subscribe<Event extends RendererEventEnvelope>(
    handler: RendererEventHandler<Event>,
  ) {
    const registered = handler as RendererEventHandler<RendererEventEnvelope>;
    handlers.add(registered);
    return () => handlers.delete(registered);
  }

  function subscribeNamespace<Event extends RendererEventEnvelope>(
    namespace: Event["namespace"],
    handler: RendererEventHandler<Event>,
  ) {
    return subscribe((event) => {
      if (event.namespace === namespace) {
        handler(event as Event);
      }
    });
  }

  return {
    dispatch,
    subscribe,
    subscribeNamespace,
  };
}

let nextRendererEventSequence = 1;

export function rendererEvent<
  Namespace extends string,
  EventKey extends string,
  Payload,
>(
  namespace: Namespace,
  eventkey: EventKey,
  payload: Payload,
): RendererEventEnvelope<Namespace, EventKey, Payload> {
  const sequence = nextRendererEventSequence;
  nextRendererEventSequence += 1;

  return {
    namespace,
    eventkey,
    payload,
    event_id: `renderer-${Date.now()}-${sequence}`,
    emitted_at: new Date().toISOString(),
  };
}
