package dev.allwright.client;

import io.grpc.ManagedChannel;
import io.grpc.stub.StreamObserver;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.LinkedBlockingQueue;

final class RuntimeSupport {
    private RuntimeSupport() {}

    record RuntimeClient(
            ManagedChannel channel,
            dev.allwright.engine.v1.EngineServiceGrpc.EngineServiceBlockingStub blockingStub,
            dev.allwright.engine.v1.EngineServiceGrpc.EngineServiceStub asyncStub
    ) {}

    static final class StreamHandle<RequestT, ResponseT> {
        private final EventQueue<ResponseT> events = new EventQueue<>();
        private final StreamObserver<RequestT> requests;
        private boolean sendClosed;

        StreamHandle(StreamFactory<RequestT, ResponseT> streamFactory) {
            this.requests = streamFactory.open(new StreamObserver<>() {
                @Override
                public void onNext(ResponseT value) {
                    events.push(value);
                }

                @Override
                public void onError(Throwable throwable) {
                    events.fail(throwable);
                }

                @Override
                public void onCompleted() {
                    events.complete();
                }
            });
        }

        void send(RequestT message) {
            if (sendClosed) {
                throw new AllwrightException("cannot send on a closed stream");
            }
            try {
                requests.onNext(message);
            } catch (RuntimeException exception) {
                throw new AllwrightException("send stream command: " + exception.getMessage(), exception);
            }
        }

        ResponseT recv(String action) {
            return events.next(action);
        }

        void closeSend() {
            if (sendClosed) {
                return;
            }
            sendClosed = true;
            requests.onCompleted();
        }
    }

    private static final class EventQueue<T> {
        private final BlockingQueue<EventOrFailure<T>> items = new LinkedBlockingQueue<>();

        private void push(T value) {
            items.add(EventOrFailure.event(value));
        }

        private void fail(Throwable throwable) {
            items.add(EventOrFailure.failure(throwable));
        }

        private void complete() {
            items.add(EventOrFailure.completed());
        }

        private T next(String action) {
            try {
                EventOrFailure<T> item = items.take();
                if (item.failure != null) {
                    throw new AllwrightException(action + ": " + item.failure.getMessage(), item.failure);
                }
                if (item.completed) {
                    throw new AllwrightException(action + ": stream ended unexpectedly");
                }
                return item.event;
            } catch (InterruptedException exception) {
                Thread.currentThread().interrupt();
                throw new AllwrightException(action + ": interrupted while waiting for stream event", exception);
            }
        }
    }

    private static final class EventOrFailure<T> {
        private final T event;
        private final Throwable failure;
        private final boolean completed;

        private EventOrFailure(T event, Throwable failure, boolean completed) {
            this.event = event;
            this.failure = failure;
            this.completed = completed;
        }

        private static <T> EventOrFailure<T> event(T value) {
            return new EventOrFailure<>(value, null, false);
        }

        private static <T> EventOrFailure<T> failure(Throwable throwable) {
            return new EventOrFailure<>(null, throwable, false);
        }

        private static <T> EventOrFailure<T> completed() {
            return new EventOrFailure<>(null, null, true);
        }
    }

    @FunctionalInterface
    interface StreamFactory<RequestT, ResponseT> {
        StreamObserver<RequestT> open(StreamObserver<ResponseT> responseObserver);
    }
}
