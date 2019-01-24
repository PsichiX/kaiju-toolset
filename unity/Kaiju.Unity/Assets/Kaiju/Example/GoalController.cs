using UnityEngine;

public class GoalController : MonoBehaviour
{
    [SerializeField]
    private GameObject m_win;

    private void OnTriggerEnter(Collider other)
    {
        var player = other.GetComponent<PlayerController>();
        if (player != null && m_win != null)
        {
            m_win.SetActive(true);
        }
    }

    public void OnClickRestart()
    {
        if (m_win != null)
        {
            m_win.SetActive(false);
        }
    }
}
