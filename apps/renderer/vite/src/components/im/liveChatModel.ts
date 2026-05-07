import type {
  ConversationToolActivity,
  ControllerOperationEvent,
  FirstMessageResponse,
  TranscriptMessage,
} from "../../controller/client";
import {
  type RendererEventBus,
  type RendererEventEnvelope,
  rendererEvent,
} from "../../events/eventbus";

import {
  createChatMessage,
  initialLiveMessages,
  textContent,
} from "./sessionModel";
import type { ChatMessage } from "./types";

type ChatEventMap = {
  "conversation.reset": Record<string, never>;
  "conversation.load-failed": {
    conversation_id: string;
    message: string;
  };
  "transcript.loaded": {
    conversation_id: string;
    messages: TranscriptMessage[];
    tool_activities: ConversationToolActivity[];
  };
  "message.optimistic-created": {
    message_id: string;
    text: string;
  };
  "message.delivery-failed": {
    message_id: string;
    meta_label: string;
  };
  "roundtrip.completed": {
    response: FirstMessageResponse;
    pending_message_id: string;
  };
  "controller.operation-event": {
    event: ControllerOperationEvent;
  };
  "error.changed": {
    message: string | null;
  };
};

export type ChatEventKey = keyof ChatEventMap;

export type ChatRendererEvent<Key extends ChatEventKey = ChatEventKey> = {
  [EventKey in ChatEventKey]: RendererEventEnvelope<
    "chat",
    EventKey,
    ChatEventMap[EventKey]
  >;
}[Key];

export type LiveChatModel = {
  activeConversationId: string | null;
  messages: ChatMessage[];
  toolActivities: ConversationToolActivity[];
  sendResult: FirstMessageResponse | null;
  lastResponseText: string | null;
  lastResponseSource: string | null;
  lastFinalSentText: string | null;
  optimisticMessageId: string | null;
  errorMessage: string | null;
};

export function createLiveChatModel(
  storedConversationId: string | null,
): LiveChatModel {
  return {
    activeConversationId: storedConversationId,
    messages: storedConversationId ? [] : initialLiveMessages(),
    toolActivities: [],
    sendResult: null,
    lastResponseText: null,
    lastResponseSource: null,
    lastFinalSentText: null,
    optimisticMessageId: null,
    errorMessage: null,
  };
}

export function dispatchChatEvent<Key extends ChatEventKey>(
  eventbus: RendererEventBus,
  eventkey: Key,
  payload: ChatEventMap[Key],
) {
  eventbus.dispatch(rendererEvent("chat", eventkey, payload));
}

export function applyChatEvent(model: LiveChatModel, event: ChatRendererEvent) {
  switch (event.eventkey) {
    case "conversation.reset":
      model.activeConversationId = null;
      model.messages = [];
      model.toolActivities = [];
      model.sendResult = null;
      model.lastResponseText = null;
      model.lastResponseSource = null;
      model.lastFinalSentText = null;
      model.optimisticMessageId = null;
      model.errorMessage = null;
      return;
    case "conversation.load-failed":
      model.activeConversationId = null;
      model.messages = [];
      model.toolActivities = [];
      model.sendResult = null;
      model.lastResponseText = null;
      model.lastResponseSource = null;
      model.lastFinalSentText = null;
      model.optimisticMessageId = null;
      model.errorMessage = event.payload.message;
      return;
    case "transcript.loaded":
      model.activeConversationId = event.payload.conversation_id;
      model.messages = event.payload.messages.map(mapTranscriptMessage);
      model.toolActivities = event.payload.tool_activities;
      model.errorMessage = null;
      return;
    case "message.optimistic-created":
      model.optimisticMessageId = event.payload.message_id;
      model.messages.push(
        createChatMessage(
          event.payload.message_id,
          "user",
          "You",
          "Now",
          textContent(event.payload.text),
          {
            deliveryState: "sending",
          },
        ),
      );
      model.errorMessage = null;
      return;
    case "message.delivery-failed":
      model.messages = model.messages.map((message) =>
        message.id === event.payload.message_id
          ? {
              ...message,
              deliveryState: "failed",
              metaLabel: event.payload.meta_label,
            }
          : message,
      );
      if (model.optimisticMessageId === event.payload.message_id) {
        model.optimisticMessageId = null;
      }
      return;
    case "roundtrip.completed":
      applyRoundtripFallback(
        model,
        event.payload.response,
        event.payload.pending_message_id,
      );
      return;
    case "controller.operation-event":
      applyControllerOperationEvent(model, event.payload.event);
      return;
    case "error.changed":
      model.errorMessage = event.payload.message;
      return;
  }
}

function applyRoundtripFallback(
  model: LiveChatModel,
  response: FirstMessageResponse,
  pendingId: string,
) {
  model.sendResult = response;
  model.activeConversationId = response.conversation_id;
  model.lastResponseText = response.response_text;
  model.lastResponseSource = response.response_text_source;
  model.lastFinalSentText = response.final_sent_text;
  model.messages = model.messages.map((message) =>
    message.id === pendingId
      ? createChatMessage(
          `${response.message_id}-user`,
          "user",
          "You",
          "Now",
          response.final_sent_content,
          {
            deliveryState: "sent",
            metaLabel: "Delivered to controller",
          },
        )
      : message,
  );
  model.messages.push(
    createChatMessage(
      `${response.message_id}-assistant`,
      "assistant",
      "stim",
      "Now",
      response.response_content,
      {
        metaLabel: "Controller reply",
      },
    ),
  );
  model.optimisticMessageId = null;
  model.errorMessage = null;
}

function applyControllerOperationEvent(
  model: LiveChatModel,
  event: ControllerOperationEvent,
) {
  if (event.conversation_id) {
    model.activeConversationId = event.conversation_id;
  }

  if (event.stage === "delivery-started" && event.message_id) {
    upsertAssistantPendingMessage(model, event.message_id, event.detail);
  }

  if (event.stage === "message-chunk-appended" && event.message_delta) {
    appendAssistantMessageDelta(model, event.message_delta.message_id, event.message_delta.text);
  }

  if (event.snapshot) {
    model.activeConversationId = event.snapshot.conversation_id;
    model.messages = event.snapshot.messages.map((message) =>
      createChatMessage(
        message.id,
        message.role,
        message.role === "user" ? "You" : "stim",
        "Now",
        textContent(message.text),
        {
          deliveryState: message.role === "user" ? "sent" : undefined,
          metaLabel: message.role === "assistant" ? "Controller reply" : null,
        },
      ),
    );
    model.toolActivities = event.snapshot.tool_activities;
    model.lastResponseText = event.snapshot.last_assistant_text;
    model.lastResponseSource = event.snapshot.response_text_source;
    model.lastFinalSentText = event.snapshot.final_sent_text;
    model.optimisticMessageId = null;
  }

  if (event.stage === "operation-failed" || event.status === "failed") {
    model.errorMessage = event.detail ?? "Controller operation failed";
  } else {
    model.errorMessage = null;
  }
}

function appendAssistantMessageDelta(
  model: LiveChatModel,
  messageId: string,
  text: string,
) {
  const existingIndex = model.messages.findIndex(
    (message) => message.id === messageId,
  );
  if (existingIndex >= 0) {
    const existing = model.messages[existingIndex];
    model.messages[existingIndex] = {
      ...existing,
      content: textContent(`${textFromContent(existing.content)}${text}`),
      metaLabel: "Controller reply streaming",
    };
  } else {
    const pendingIndex = model.messages.findIndex(
      (message) => message.id === pendingAssistantIdFor(messageId),
    );
    const streamedMessage = createChatMessage(
      messageId,
      "assistant",
      "stim",
      "Now",
      textContent(text),
      {
        metaLabel: "Controller reply streaming",
      },
    );
    if (pendingIndex >= 0) {
      model.messages.splice(pendingIndex, 1, streamedMessage);
    } else {
      model.messages.push(streamedMessage);
    }
  }

  model.lastResponseText = `${model.lastResponseText ?? ""}${text}`;
  model.lastResponseSource = "stim_reply_stream";
  model.optimisticMessageId = null;
}

function upsertAssistantPendingMessage(
  model: LiveChatModel,
  userMessageId: string,
  detail: string | null,
) {
  const pendingAssistantId = `${userMessageId}-assistant-pending`;
  if (model.messages.some((message) => message.id === pendingAssistantId)) {
    return;
  }

  model.messages.push(
    createChatMessage(
      pendingAssistantId,
      "assistant",
      "stim",
      "Now",
      textContent("..."),
      {
        metaLabel: detail ?? "Controller operation running",
      },
    ),
  );
}

function pendingAssistantIdFor(messageId: string): string {
  return `${messageId}-pending`;
}

function textFromContent(content: ChatMessage["content"]): string {
  return content.parts
    .map((part) => {
      if (part.kind === "text") {
        return part.text;
      }

      return "";
    })
    .join("");
}

function mapTranscriptMessage(message: TranscriptMessage): ChatMessage {
  return createChatMessage(
    message.id,
    message.role,
    message.author,
    message.sent_at_label,
    message.content,
    {
      deliveryState: message.delivery_state ?? undefined,
      metaLabel: message.meta_label,
    },
  );
}
