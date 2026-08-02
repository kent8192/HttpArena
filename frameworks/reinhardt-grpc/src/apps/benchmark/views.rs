//! gRPC handlers for the benchmark app.

use std::pin::Pin;
use std::sync::Arc;

use reinhardt::grpc::grpc_handler;
use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::benchmark::benchmark_service_server::BenchmarkService;
use crate::benchmark::{StreamRequest, SumReply, SumRequest};

type ReplyStream = Pin<Box<dyn Stream<Item = Result<SumReply, Status>> + Send + 'static>>;

#[derive(Clone, Copy)]
struct SumCalculator;

#[reinhardt_di::async_trait::async_trait]
impl Injectable for SumCalculator {
    async fn inject(_context: &InjectionContext) -> DiResult<Self> {
        Ok(Self)
    }
}

impl SumCalculator {
    fn sum(self, a: i32, b: i32) -> i32 {
        a + b
    }
}

pub struct BenchmarkServiceImpl {
    injection_context: Arc<InjectionContext>,
}

impl Default for BenchmarkServiceImpl {
    fn default() -> Self {
        let singleton_scope = Arc::new(SingletonScope::new());
        Self {
            injection_context: Arc::new(InjectionContext::builder(singleton_scope).build()),
        }
    }
}

impl BenchmarkServiceImpl {
    fn with_context<T>(&self, mut request: Request<T>) -> Request<T> {
        request
            .extensions_mut()
            .insert(self.injection_context.clone());
        request
    }

    #[grpc_handler]
    async fn get_sum_handler(
        &self,
        request: Request<SumRequest>,
        #[inject] calculator: SumCalculator,
    ) -> Result<Response<SumReply>, Status> {
        let request = request.into_inner();
        Ok(Response::new(SumReply {
            result: calculator.sum(request.a, request.b),
        }))
    }

    #[grpc_handler]
    async fn stream_sum_handler(
        &self,
        request: Request<StreamRequest>,
        #[inject] calculator: SumCalculator,
    ) -> Result<Response<ReplyStream>, Status> {
        let request = request.into_inner();
        let count = request.count.max(0) as usize;
        let result = calculator.sum(request.a, request.b);
        let replies = tokio_stream::iter((0..count).map(move |_| Ok(SumReply { result })));
        Ok(Response::new(Box::pin(replies)))
    }

    #[grpc_handler]
    async fn collect_sum_handler(
        &self,
        request: Request<Streaming<SumRequest>>,
        #[inject] calculator: SumCalculator,
    ) -> Result<Response<SumReply>, Status> {
        let mut stream = request.into_inner();
        let mut result = 0;
        while let Some(request) = stream.next().await {
            let request = request?;
            result += calculator.sum(request.a, request.b);
        }
        Ok(Response::new(SumReply { result }))
    }

    #[grpc_handler]
    async fn echo_sum_handler(
        &self,
        request: Request<Streaming<SumRequest>>,
        #[inject] calculator: SumCalculator,
    ) -> Result<Response<ReplyStream>, Status> {
        let replies = request.into_inner().map(move |request| {
            request.map(|request| SumReply {
                result: calculator.sum(request.a, request.b),
            })
        });
        Ok(Response::new(Box::pin(replies)))
    }
}

#[tonic::async_trait]
impl BenchmarkService for BenchmarkServiceImpl {
    async fn get_sum(&self, request: Request<SumRequest>) -> Result<Response<SumReply>, Status> {
        self.get_sum_handler(self.with_context(request)).await
    }

    type StreamSumStream = ReplyStream;

    async fn stream_sum(
        &self,
        request: Request<StreamRequest>,
    ) -> Result<Response<Self::StreamSumStream>, Status> {
        self.stream_sum_handler(self.with_context(request)).await
    }

    async fn collect_sum(
        &self,
        request: Request<Streaming<SumRequest>>,
    ) -> Result<Response<SumReply>, Status> {
        self.collect_sum_handler(self.with_context(request)).await
    }

    type EchoSumStream = ReplyStream;

    async fn echo_sum(
        &self,
        request: Request<Streaming<SumRequest>>,
    ) -> Result<Response<Self::EchoSumStream>, Status> {
        self.echo_sum_handler(self.with_context(request)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unary_sum_uses_both_operands() {
        let response = BenchmarkServiceImpl::default()
            .get_sum(Request::new(SumRequest { a: 13, b: 42 }))
            .await
            .unwrap();
        assert_eq!(response.into_inner().result, 55);
    }

    #[tokio::test]
    async fn stream_sum_returns_requested_number_of_identical_replies() {
        let response = BenchmarkServiceImpl::default()
            .stream_sum(Request::new(StreamRequest {
                a: 13,
                b: 42,
                count: 3,
            }))
            .await
            .unwrap();
        let replies: Vec<_> = response.into_inner().collect().await;
        assert_eq!(replies.len(), 3);
        assert!(replies.into_iter().all(|reply| reply.unwrap().result == 55));
    }

    #[tokio::test]
    async fn grpc_handler_requires_an_injection_context() {
        let response = BenchmarkServiceImpl::default()
            .get_sum_handler(Request::new(SumRequest { a: 13, b: 42 }))
            .await;
        assert_eq!(response.unwrap_err().code(), tonic::Code::Internal);
    }
}
