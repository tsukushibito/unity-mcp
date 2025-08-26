// Unity MCP Bridge - EditorDispatcher
// Provides a unified way to execute code on Unity's main thread from background contexts
using System;
using System.Collections.Concurrent;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Mcp.Unity.V1.Ipc.Infra
{
    /// <summary>
    /// Provides safe execution of Unity API calls on the main thread from background contexts.
    /// Uses a queued work model driven by EditorApplication.update.
    /// </summary>
    internal static class EditorDispatcher
    {
        private static readonly ConcurrentQueue<Func<Task>> _workQueue = new ConcurrentQueue<Func<Task>>();
        private const int MaxItemsPerFrame = 256; // Prevent frame stalls
        private const float MaxTimeSliceMs = 2.0f; // 2ms time budget per frame

        /// <summary>
        /// Initialize the dispatcher when Unity Editor loads
        /// </summary>
        [InitializeOnLoadMethod]
        private static void Initialize()
        {
            // Handle assembly reload scenarios
            AssemblyReloadEvents.beforeAssemblyReload += OnBeforeAssemblyReload;
            AssemblyReloadEvents.afterAssemblyReload += OnAfterAssemblyReload;
            
            // Drive the work queue from main thread
            EditorApplication.update += Pump;
            
#if UNITY_EDITOR && DEBUG
            Diag.Log("EditorDispatcher initialized");
#endif
        }

        /// <summary>
        /// Clear pending work items before assembly reload to avoid orphaned tasks
        /// </summary>
        private static void OnBeforeAssemblyReload()
        {
#if UNITY_EDITOR && DEBUG
            int cleared = 0;
            while (_workQueue.TryDequeue(out var _)) { cleared++; }
            if (cleared > 0)
                Diag.Log($"EditorDispatcher cleared {cleared} pending items before assembly reload");
#else
            // Best-effort clear to avoid orphaned TaskCompletionSource instances
            while (_workQueue.TryDequeue(out var _)) { }
#endif
        }

        /// <summary>
        /// Re-initialize after assembly reload
        /// </summary>
        private static void OnAfterAssemblyReload()
        {
            // Nothing specific needed - Initialize() will be called again
#if UNITY_EDITOR && DEBUG
            Diag.Log("EditorDispatcher re-initialized after assembly reload");
#endif
        }

        /// <summary>
        /// Process work items from the queue on the main thread.
        /// Called from EditorApplication.update.
        /// </summary>
        private static void Pump()
        {
            if (_workQueue.IsEmpty) return;

            var processed = 0;
            var startTime = Time.realtimeSinceStartup;
            var frameDeadline = startTime + (MaxTimeSliceMs / 1000f);

            while (_workQueue.TryDequeue(out var workItem))
            {
                try
                {
                    // Execute the work item (this should be fast)
                    _ = workItem();
                }
                catch (Exception ex)
                {
                    // Log exceptions but don't let them crash the pump
                    Debug.LogException(ex);
                }

                processed++;

                // Respect frame budget to avoid stuttering
                if (processed >= MaxItemsPerFrame || Time.realtimeSinceStartup > frameDeadline)
                {
#if UNITY_EDITOR && DEBUG
                    if (processed >= MaxItemsPerFrame)
                        Diag.Log($"EditorDispatcher hit item limit ({MaxItemsPerFrame}) per frame");
                    else
                        Diag.Log($"EditorDispatcher hit time limit ({MaxTimeSliceMs}ms) per frame");
#endif
                    break;
                }
            }

#if UNITY_EDITOR && DEBUG
            if (processed > 0)
            {
                var elapsed = (Time.realtimeSinceStartup - startTime) * 1000f;
                Diag.Log($"EditorDispatcher processed {processed} items in {elapsed:F2}ms");
            }
#endif
        }

        /// <summary>
        /// Execute an action on Unity's main thread.
        /// </summary>
        /// <param name="action">Action to execute on main thread</param>
        /// <returns>Task that completes when the action finishes</returns>
        public static Task RunOnMainAsync(Action action)
        {
            if (action == null)
                throw new ArgumentNullException(nameof(action));

            return RunOnMainAsync<object>(() =>
            {
                action();
                return null;
            });
        }

        /// <summary>
        /// Execute a function on Unity's main thread and return its result.
        /// </summary>
        /// <typeparam name="T">Return type</typeparam>
        /// <param name="func">Function to execute on main thread</param>
        /// <returns>Task containing the function's result</returns>
        public static Task<T> RunOnMainAsync<T>(Func<T> func)
        {
            if (func == null)
                throw new ArgumentNullException(nameof(func));

            var tcs = new TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously);
            
            _workQueue.Enqueue(() =>
            {
                try
                {
                    var result = func();
                    tcs.SetResult(result);
                }
                catch (Exception ex)
                {
                    tcs.SetException(ex);
                }
                return Task.CompletedTask;
            });

            return tcs.Task;
        }

        /// <summary>
        /// Execute an async function on Unity's main thread and return its result.
        /// </summary>
        /// <typeparam name="T">Return type</typeparam>
        /// <param name="func">Async function to execute on main thread</param>
        /// <returns>Task containing the function's result</returns>
        public static Task<T> RunOnMainAsync<T>(Func<Task<T>> func)
        {
            if (func == null)
                throw new ArgumentNullException(nameof(func));

            var tcs = new TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously);
            
            _workQueue.Enqueue(async () =>
            {
                try
                {
                    var task = func();
                    if (task == null)
                        throw new InvalidOperationException("Function returned null task");
                    var result = await task.ConfigureAwait(false);
                    tcs.SetResult(result);
                }
                catch (Exception ex)
                {
                    tcs.SetException(ex);
                }
            });

            return tcs.Task;
        }

#if UNITY_EDITOR && DEBUG
        /// <summary>
        /// Get diagnostic information about the dispatcher state
        /// </summary>
        public static (int QueueLength, bool IsRunning) GetDiagnostics()
        {
            // Note: ConcurrentQueue.Count can be expensive, only use in debug
            var queueLength = 0;
            var tempQueue = new ConcurrentQueue<Func<Task>>();
            
            // Count items by dequeuing and re-enqueuing
            while (_workQueue.TryDequeue(out var item))
            {
                tempQueue.Enqueue(item);
                queueLength++;
            }
            
            // Restore items
            while (tempQueue.TryDequeue(out var item))
            {
                _workQueue.Enqueue(item);
            }
            
            return (queueLength, EditorApplication.isPlaying || EditorApplication.isCompiling == false);
        }
#endif
    }
}