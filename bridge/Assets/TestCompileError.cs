#if false
using UnityEngine;

public class TestCompileError : MonoBehaviour
{
    void Start()
    {
        // コンパイルエラーを意図的に作成
        UndefinedVariable = 42; // CS0103: 'UndefinedVariable' が存在しない

        string text = 42; // CS0029: 型変換エラー

        Debug.Log("This will not compile");

        // 使用されていない変数警告
        int unusedVariable = 100; // CS0219: 警告
    }

    void UnusedMethod() // CS0169: 警告 - 使用されていないメソッド
    {
        NonExistentFunction(); // CS0103: 存在しない関数
    }
}
#endif