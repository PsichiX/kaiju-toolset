using System;
using System.Collections;
using System.Collections.Generic;
using System.Text;
using UnityEngine;
using UnityEngine.UI;
using Compiler = Kaiju.Compiler.API;
using VM = Kaiju.VM.API;

public class PlayerController : MonoBehaviour
{
    private class FileMap : Dictionary<string, byte[]> { }

    private enum Action
    {
        None,
        MovX,
        MovY,
    }

    private const string DESCRIPTOR = "movx v: i32 {}\nmovy v: i32 {}";

    [SerializeField]
    private InputField m_code;
    [SerializeField]
    private Button m_runButton;
    [SerializeField]
    private Button m_stopButton;
    [SerializeField]
    private Rigidbody m_rigidBody;
    [SerializeField]
    private TextAsset m_codeContents;
    [SerializeField]
    private float m_speed = 1;

    private FileMap m_files = new FileMap();
    private UIntPtr? m_program;
    private Action m_action = Action.None;
    private Vector3 m_originPos;
    private Coroutine m_coroutine;

    private void Start()
    {
        m_files["descriptor.kjo"] = Encoding.UTF8.GetBytes(DESCRIPTOR);

        if (m_code != null && m_codeContents != null)
        {
            m_code.text = m_codeContents.text;
        }
        m_originPos = transform.position;
    }

    public void OnClickRun()
    {
        if (m_code == null || m_program.HasValue)
        {
            return;
        }
        m_files["program.kj"] = Encoding.UTF8.GetBytes(m_code.text);

        var assembly = Compiler.CompileBin("program.kj", "descriptor.kjo", m_files, error => Debug.LogError(error));
        if (assembly == null)
        {
            return;
        }

        m_program = VM.Start(assembly, "main", 256, 256, error => Debug.LogError(error));
        m_action = Action.None;
        if (m_runButton != null)
        {
            m_runButton.interactable = false;
        }
        if (m_stopButton != null)
        {
            m_stopButton.interactable = true;
        }
    }

    public void OnClickStop()
    {
        if (m_program.HasValue)
        {
            VM.Cancel(m_program.Value);
            m_program = null;
            m_action = Action.None;
            if (m_runButton != null)
            {
                m_runButton.interactable = true;
            }
            if (m_stopButton != null)
            {
                m_stopButton.interactable = false;
            }
            transform.position = m_originPos;
            transform.rotation = Quaternion.identity;
            if (m_coroutine != null)
            {
                StopCoroutine(m_coroutine);
            }
        }
    }

    private void Update()
    {
        if (!m_program.HasValue)
        {
            return;
        }
        if (m_action == Action.None)
        {
            if (!VM.Resume(m_program.Value, OnProcessOp, error => Debug.LogError(error)))
            {
                m_program = null;
                m_action = Action.None;
                if (m_runButton != null)
                {
                    m_runButton.interactable = true;
                }
                if (m_stopButton != null)
                {
                    m_stopButton.interactable = false;
                }
                transform.position = m_originPos;
                transform.rotation = Quaternion.identity;
            }
        }
    }

    private void OnProcessOp(string op, UIntPtr[] paramsPtrs, UIntPtr[] targetsPtrs)
    {
        if (op == "movx")
        {
            m_action = Action.MovX;
            var value = VM.StateLoad<int>(paramsPtrs[0]);
            m_coroutine = StartCoroutine(Move(Mathf.Sign((float)value), 0, Mathf.Abs((float)value)));
        }
        else if (op == "movy")
        {
            m_action = Action.MovY;
            var value = VM.StateLoad<int>(paramsPtrs[0]);
            m_coroutine = StartCoroutine(Move(0, Mathf.Sign((float)-value), Mathf.Abs((float)value)));
        }
    }

    private IEnumerator Move(float x, float y, float time)
    {
        if (m_rigidBody == null)
        {
            m_action = Action.None;
            yield break;
        }
        while (time > 0)
        {
            var dt = Time.smoothDeltaTime * m_speed;
            time -= dt;
            m_rigidBody.MovePosition(m_rigidBody.position + new Vector3(x, 0, y) * dt);
            yield return new WaitForEndOfFrame();
        }
        m_action = Action.None;
        m_coroutine = null;
    }
}
