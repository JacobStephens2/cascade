using System;
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

    private void OnStartCustom(object sender, RoutedEventArgs e)
    {
        // NumberBox.Value is NaN when the field is empty / invalid.
        var raw = CustomMinutes.Value;
        if (double.IsNaN(raw)) return;
        var minutes = (int)Math.Round(raw);
        if (minutes < 1 || minutes > 1440) return;
        ViewModel.StartCustom(minutes, sleep: CustomSleepRadio.IsChecked == true);
    }
}
