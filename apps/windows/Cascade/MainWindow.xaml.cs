using Cascade.Services;
using Cascade.ViewModels;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace Cascade;

public sealed partial class MainWindow : Window
{
    public AppViewModel ViewModel { get; }

    public MainWindow()
    {
        InitializeComponent();
        ViewModel = new AppViewModel(DispatcherQueue.GetForCurrentThread());
        Closed += (_, _) => ViewModel.Dispose();
    }

    /// <summary>
    /// Slider raises ValueChanged on every motion frame; ignore the initial
    /// load callback (where IntermediateValue == OldValue) so we don't
    /// dispatch a `setVolume` on the snapshot's initial bind.
    /// </summary>
    private void OnVolumeChanged(object sender, RangeBaseValueChangedEventArgs e)
    {
        if (sender is Slider slider && slider.IsLoaded)
        {
            ViewModel.SetVolumeCommand.Execute(e.NewValue);
        }
    }
}
