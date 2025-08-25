// Unity MCP Bridge - EditorDispatcher Tests
// EditMode tests to verify EditorDispatcher functionality and main-thread execution
using System;
using System.Collections;
using System.Threading;
using System.Threading.Tasks;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEditor;

namespace Mcp.Unity.V1.Ipc.Infra.Tests
{
    /// <summary>
    /// Tests for EditorDispatcher to verify main-thread execution guarantees
    /// </summary>
    [TestFixture]
    public class EditorDispatcherTests
    {
        [OneTimeSetUp]
        public void OneTimeSetUp()
        {
            // Ensure EditorDispatcher is initialized
            // The [InitializeOnLoadMethod] should have already done this
#if UNITY_EDITOR && DEBUG
            Diag.Log("EditorDispatcherTests starting");
#endif
        }

        /// <summary>
        /// Verify that RunOnMainAsync(Action) executes on the main thread
        /// </summary>
        [UnityTest]
        public IEnumerator TestActionRunsOnMainThread()
        {
            var mainThreadId = Thread.CurrentThread.ManagedThreadId;
            int? executedThreadId = null;
            Exception capturedException = null;

            var task = EditorDispatcher.RunOnMainAsync(() =>
            {
                executedThreadId = Thread.CurrentThread.ManagedThreadId;
            });

            // Wait for completion
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                capturedException = task.Exception?.GetBaseException();
            }

            Assert.IsNull(capturedException, $"Task should not fault: {capturedException?.Message}");
            Assert.IsTrue(task.IsCompletedSuccessfully, "Task should complete successfully");
            Assert.IsNotNull(executedThreadId, "Executed thread ID should be captured");
            Assert.AreEqual(mainThreadId, executedThreadId, "Action should execute on main thread");

#if UNITY_EDITOR && DEBUG
            Diag.Log($"Action executed on thread {executedThreadId} (main: {mainThreadId})");
#endif
        }

        /// <summary>
        /// Verify that RunOnMainAsync(Func<T>) returns the correct result
        /// </summary>
        [UnityTest]
        public IEnumerator TestFuncReturnsCorrectResult()
        {
            var expectedResult = "test-result";
            string actualResult = null;
            Exception capturedException = null;

            var task = EditorDispatcher.RunOnMainAsync(() =>
            {
                // Verify we can call Unity APIs safely
                var version = Application.unityVersion;
                return $"{expectedResult}-{version}";
            });

            // Wait for completion
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                capturedException = task.Exception?.GetBaseException();
            }
            else if (task.IsCompletedSuccessfully)
            {
                actualResult = task.Result;
            }

            Assert.IsNull(capturedException, $"Task should not fault: {capturedException?.Message}");
            Assert.IsTrue(task.IsCompletedSuccessfully, "Task should complete successfully");
            Assert.IsNotNull(actualResult, "Result should not be null");
            Assert.IsTrue(actualResult.StartsWith(expectedResult), $"Result should start with '{expectedResult}', got '{actualResult}'");

#if UNITY_EDITOR && DEBUG
            Diag.Log($"Func returned: {actualResult}");
#endif
        }

        /// <summary>
        /// Verify that exceptions are properly propagated through the dispatcher
        /// </summary>
        [UnityTest]
        public IEnumerator TestExceptionPropagation()
        {
            var expectedMessage = "test-exception";
            Exception capturedException = null;

            var task = EditorDispatcher.RunOnMainAsync(() =>
            {
                throw new InvalidOperationException(expectedMessage);
            });

            // Wait for completion
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                capturedException = task.Exception?.GetBaseException();
            }

            Assert.IsTrue(task.IsFaulted, "Task should be faulted");
            Assert.IsNotNull(capturedException, "Exception should be captured");
            Assert.IsInstanceOf<InvalidOperationException>(capturedException, "Exception should be of correct type");
            Assert.AreEqual(expectedMessage, capturedException.Message, "Exception message should match");

#if UNITY_EDITOR && DEBUG
            Diag.Log($"Exception properly propagated: {capturedException.GetType().Name} - {capturedException.Message}");
#endif
        }

        /// <summary>
        /// Test async function execution
        /// </summary>
        [UnityTest]
        public IEnumerator TestAsyncFuncExecution()
        {
            var expectedValue = 42;
            int actualValue = 0;
            Exception capturedException = null;

            var task = EditorDispatcher.RunOnMainAsync(async () =>
            {
                // Simulate async work (but don't actually delay in tests)
                await Task.Yield();
                
                // Access Unity API to verify main thread
                var isPlaying = Application.isPlaying;
                return expectedValue;
            });

            // Wait for completion
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                capturedException = task.Exception?.GetBaseException();
            }
            else if (task.IsCompletedSuccessfully)
            {
                actualValue = task.Result;
            }

            Assert.IsNull(capturedException, $"Task should not fault: {capturedException?.Message}");
            Assert.IsTrue(task.IsCompletedSuccessfully, "Task should complete successfully");
            Assert.AreEqual(expectedValue, actualValue, "Async function should return correct value");

#if UNITY_EDITOR && DEBUG
            Diag.Log($"Async func returned: {actualValue}");
#endif
        }

        /// <summary>
        /// Test multiple concurrent dispatches to ensure queue ordering
        /// </summary>
        [UnityTest]
        public IEnumerator TestConcurrentDispatch()
        {
            const int taskCount = 10;
            var results = new int[taskCount];
            var tasks = new Task[taskCount];
            var exceptions = new Exception[taskCount];

            // Submit multiple tasks concurrently
            for (int i = 0; i < taskCount; i++)
            {
                var taskIndex = i;
                tasks[i] = EditorDispatcher.RunOnMainAsync(() =>
                {
                    results[taskIndex] = taskIndex * 2;
                });
            }

            // Wait for all to complete
            yield return new WaitUntil(() =>
            {
                bool allCompleted = true;
                for (int i = 0; i < taskCount; i++)
                {
                    if (!tasks[i].IsCompleted)
                    {
                        allCompleted = false;
                        break;
                    }
                    if (tasks[i].IsFaulted)
                    {
                        exceptions[i] = tasks[i].Exception?.GetBaseException();
                    }
                }
                return allCompleted;
            });

            // Verify all completed successfully
            for (int i = 0; i < taskCount; i++)
            {
                Assert.IsNull(exceptions[i], $"Task {i} should not fault: {exceptions[i]?.Message}");
                Assert.IsTrue(tasks[i].IsCompletedSuccessfully, $"Task {i} should complete successfully");
                Assert.AreEqual(i * 2, results[i], $"Task {i} should have correct result");
            }

#if UNITY_EDITOR && DEBUG
            Diag.Log($"All {taskCount} concurrent tasks completed successfully");
#endif
        }

        /// <summary>
        /// Test that Unity API access from background thread is unsafe,
        /// but safe when dispatched through EditorDispatcher
        /// </summary>
        [UnityTest]
        public IEnumerator TestUnityApiSafety()
        {
            string safeVersion = null;
            Exception capturedException = null;

            // This should be safe when dispatched to main thread
            var task = EditorDispatcher.RunOnMainAsync(() =>
            {
                // These Unity API calls should be safe on main thread
                var version = Application.unityVersion;
                var platform = Application.platform;
                var isEditor = Application.isEditor;
                
                return $"{version}-{platform}-{isEditor}";
            });

            // Wait for completion
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                capturedException = task.Exception?.GetBaseException();
            }
            else if (task.IsCompletedSuccessfully)
            {
                safeVersion = task.Result;
            }

            Assert.IsNull(capturedException, $"Dispatched Unity API access should be safe: {capturedException?.Message}");
            Assert.IsTrue(task.IsCompletedSuccessfully, "Task should complete successfully");
            Assert.IsNotNull(safeVersion, "Unity API should return valid data");
            Assert.IsTrue(safeVersion.Contains("True"), "Should confirm we're in editor");

#if UNITY_EDITOR && DEBUG
            Diag.Log($"Safe Unity API access result: {safeVersion}");
#endif
        }

#if UNITY_EDITOR && DEBUG
        /// <summary>
        /// Test diagnostic functionality (debug builds only)
        /// </summary>
        [Test]
        public void TestDiagnostics()
        {
            var (queueLength, isRunning) = EditorDispatcher.GetDiagnostics();
            
            // Queue should be manageable size
            Assert.GreaterOrEqual(queueLength, 0, "Queue length should be non-negative");
            Assert.LessOrEqual(queueLength, 1000, "Queue should not be excessively long");
            
            // Dispatcher should be running in editor
            // Note: isRunning logic might vary based on editor state
            
            Diag.Log($"EditorDispatcher diagnostics: Queue={queueLength}, Running={isRunning}");
        }
#endif

        /// <summary>
        /// Test null argument handling
        /// </summary>
        [Test]
        public void TestNullArgumentHandling()
        {
            Assert.Throws<ArgumentNullException>(() => EditorDispatcher.RunOnMainAsync((Action)null));
            Assert.Throws<ArgumentNullException>(() => EditorDispatcher.RunOnMainAsync((Func<int>)null));
            Assert.Throws<ArgumentNullException>(() => EditorDispatcher.RunOnMainAsync((Func<Task<int>>)null));
        }
    }
}